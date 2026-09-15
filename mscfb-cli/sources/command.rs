//! `clap` demande trop de dépendences, pas besoin de ça pour si peu.

#[derive(Debug, Default)]
pub struct Command {
  pub name: Option<String>,
  pub compound_file: Option<String>,
  pub directory_path: Option<String>,
  pub destination: Option<String>,
}

impl Command {
  pub fn usage() {
    if let Some(executable) = std::env::current_exe().ok() {
      eprintln!("Usage {:?}:", executable.file_name());
    } else {
      eprintln!("Usage:");
    }

    eprint!(include_str!("./usage.txt"));
  }

  pub fn new() -> Command {
    let mut command = Command::default();
    let mut arguments = std::env::args().skip(1);

    loop {
      let Some(argument) = arguments.next() else {
        return command;
      };

      if argument.eq_ignore_ascii_case("-f") {
        command.compound_file = arguments.next();
        continue;
      }

      if argument.eq_ignore_ascii_case("-d") {
        command.directory_path = arguments.next();
        continue;
      }

      if argument.eq_ignore_ascii_case("-o") {
        command.destination = arguments.next();
        continue;
      }

      if command.name.is_none() {
        command.name = Some(argument);
        continue;
      }

      if command.compound_file.is_none() {
        command.compound_file = Some(argument);
        continue;
      }

      if command.directory_path.is_none() {
        command.directory_path = Some(argument);
        continue;
      }

      if command.destination.is_none() {
        command.destination = Some(argument);
        continue;
      }

      return command;
    }
  }
}
