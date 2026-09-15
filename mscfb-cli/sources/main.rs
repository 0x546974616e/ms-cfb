use mscfb_cf::{CfObject, CompoundFile, Mmap};
use mscfb_utils::InBytes;
use std::path::PathBuf;
use std::{fs, io};

mod command;
use command::Command;

/// Liste toutes les entrées d'un répertoire.
fn list_command(command: Command, cf: CompoundFile<Mmap>) {
  let directory_path = match command.directory_path {
    Some(ref directory_path) => directory_path.as_ref(),
    None => "/",
  };

  // On ouvre le répertoire.
  let object = cf.open(directory_path);
  let object = object.expect("object");

  // Si c'est pas un storage on l'affiche.
  if object.r#type().is_stream() {
    println!("{}", object.directory_entry());
    return;
  }

  // On affiche d'abord l'entrée elle-même (`.`).
  let mut current = object.directory_entry().clone();
  println!("{}", current.set_name(".").expect("."));

  // Puis toutes les entrées une à une.
  let done = object.walk_directory(|object| {
    println!("{}", object.directory_entry());
  });

  done.expect("list");
}

/// Affiche chaque entrée récursivement sous la forme d'un arbre.
fn tree_command(command: Command, cf: CompoundFile<Mmap>) {
  let directory_path = match command.directory_path {
    Some(ref directory_path) => directory_path.as_ref(),
    None => "/",
  };

  // On ouvre le répertoire.
  let object = cf.open(directory_path);
  let object = object.expect("object");

  let entry = object.directory_entry();
  let bytes = InBytes(entry.starting_sector.into());
  println!("+ {} ({bytes})", object.directory_entry().name());
  walk_directory(2, &object);

  /// Liste chaque entrée d'un répertoire récursivement.
  fn walk_directory(indent: usize, object: &CfObject<Mmap>) {
    let done = object.walk_directory(|object| {
      let entry = object.directory_entry();
      let bytes = InBytes(entry.starting_sector.into());
      println!("{:>indent$}~ {} ({bytes})", "", entry.name());

      // Puis on recommence.
      if object.r#type().is_storage() {
        walk_directory(indent + 2, object);
      }
    });
    done.expect("tree");
  }
}

/// Affiche le contenu d'un stream.
fn cat_command(command: Command, cf: CompoundFile<Mmap>) {
  let directory_path = match command.directory_path {
    Some(ref directory_path) => directory_path.as_ref(),
    None => "/",
  };

  let object = cf.open(directory_path);
  let object = object.expect("object");

  let mut stream = object.open_chain().expect("stream");
  match command.destination {
    Some(destination) => {
      // Soit dans le fichier donné.
      let mut file = fs::File::create(destination).expect("fs");
      io::copy(&mut stream, &mut file).expect("copy");
    }
    None => {
      // Sinon sur la sortie standard.
      io::copy(&mut stream, &mut io::stdout()).expect("copy");
    }
  };
}

/// Unpack le Compound File dans le système de fichiers.
fn unpack_command(command: Command, cf: CompoundFile<Mmap>) {
  let destination = command.destination.expect("destination");
  let directory_path = match command.directory_path {
    Some(ref directory_path) => directory_path.as_ref(),
    None => "/",
  };

  // On ouvre le répertoire.
  let object = cf.open(directory_path);
  let object = object.expect("object");
  unpack(&PathBuf::from(destination), &object);

  fn unpack(path: &PathBuf, object: &CfObject<Mmap>) {
    let path = if !object.r#type().is_root_storage() {
      &path.join(object.directory_entry().name().to_string())
    } else {
      path
    };

    // On traite d'abord les fichiers.
    if object.r#type().is_stream() {
      println!("{}", path.display());
      let mut stream = object.open_chain().expect("stream");
      let mut file = fs::File::create(path).expect("fs");
      io::copy(&mut stream, &mut file).expect("copy");
      return;
    }

    fs::create_dir_all(&path).expect("mkdir");
    let done = object.walk_directory(|object| {
      unpack(&path, object);
    });
    done.expect("unpack");
  }
}

fn main() {
  let command = Command::new();

  let Some(ref command_name) = command.name else {
    Command::usage();
    return;
  };

  let Some(ref compound_path) = command.compound_file else {
    Command::usage();
    return;
  };

  let compound_file = mscfb_cf::open(compound_path);
  if command_name.eq_ignore_ascii_case("list") {
    list_command(command, compound_file);
    return;
  }

  if command_name.eq_ignore_ascii_case("tree") {
    tree_command(command, compound_file);
    return;
  }

  if command_name.eq_ignore_ascii_case("cat") {
    cat_command(command, compound_file);
    return;
  }

  if command_name.eq_ignore_ascii_case("unpack") {
    unpack_command(command, compound_file);
    return;
  }

  Command::usage();
}
