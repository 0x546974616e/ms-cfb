use proc_macro::{Delimiter, Group, TokenStream, TokenTree};

#[proc_macro_derive(Pod)]
pub fn pod_derive(input: TokenStream) -> TokenStream {
  let invoke: TokenStream = "::mscfb_pod::derive_pod!".parse().unwrap();
  let arguments = TokenTree::Group(Group::new(Delimiter::Brace, input));
  invoke.into_iter().chain(Some(arguments)).collect()
}
