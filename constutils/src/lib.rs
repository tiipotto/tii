use proc_macro::TokenStream;

fn number_to_qvalue(i: u16) -> String {
  // We handle the edge cases of 0 and 1000, while using f32 for the rest.
  match i {
    0 => "0.0".to_string(),
    1000 => "1.0".to_string(),
    _ => format!("{}", f64::from(i) / 1000.0),
  }
}

#[proc_macro]
pub fn qvalue_to_strs(_input: TokenStream) -> TokenStream {
  let mut output = String::new();
  output.push_str("match self.0 {");
  for i in 0..=1000 {
    // Generate the match arm for each number
    output.push_str(&format!("{} => \"{}\",\n", i, number_to_qvalue(i)));
  }
  output.push_str("_ => unreachable!()");
  output.push_str("}");
  output.parse().unwrap()
}
