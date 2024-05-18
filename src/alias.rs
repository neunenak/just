use super::*;

/// An alias, e.g. `name := target`
#[derive(Debug, PartialEq, Clone, Serialize)]
pub(crate) struct Alias<'src, T = Rc<Recipe<'src>>> {
  pub(crate) attributes: BTreeSet<Attribute<'src>>,
  pub(crate) name: Name<'src>,
  #[serde(
    bound(serialize = "T: Keyed<'src>"),
    serialize_with = "keyed::serialize"
  )]
  pub(crate) target: T,
}

impl<'src> Alias<'src, Name<'src>> {
  pub(crate) fn resolve(self, target: Rc<Recipe<'src>>) -> Alias<'src> {
    assert_eq!(self.target.lexeme(), target.name.lexeme());

    Alias {
      attributes: self.attributes,
      name: self.name,
      target,
    }
  }
}

impl<'src, T> Alias<'src, T> {
  pub(crate) fn is_private(&self) -> bool {
    self.name.lexeme().starts_with('_') || self.attributes.contains(&Attribute::Private)
  }

  pub(crate) fn name(&self) -> &'src str {
    self.name.lexeme()
  }
}

impl<'src> Display for Alias<'src, Name<'src>> {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(
      f,
      "alias {} := {}",
      self.name.lexeme(),
      self.target.lexeme()
    )
  }
}

impl<'src> Display for Alias<'src> {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(
      f,
      "alias {} := {}",
      self.name.lexeme(),
      self.target.name.lexeme()
    )
  }
}
