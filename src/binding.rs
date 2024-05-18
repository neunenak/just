use super::*;

/// A binding of `name` to `value`
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct Binding<'src, V = String> {
  /// Module depth where binding appears
  pub(crate) depth: u32,
  /// Export binding as an environment variable to child processes
  pub(crate) export: bool,
  /// Binding name
  pub(crate) name: Name<'src>,
  /// Binding value
  pub(crate) value: V,
}

impl<'src, V> Binding<'src, V> {
  pub(crate) fn name(&self) -> &'src str {
    self.name.lexeme()
  }
}
