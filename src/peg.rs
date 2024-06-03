use super::*;

use pest::Parser;
use pest_derive::Parser;


#[derive(Parser)]
#[grammar = "justfile-grammar.pest"]
pub(crate) struct JustfileParser;

#[cfg(test)]
mod tests {

  #[test]
  fn pest_test() {
  }

}
