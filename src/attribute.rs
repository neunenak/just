use super::*;

#[derive(
  EnumDiscriminants, PartialEq, Debug, Clone, Serialize, Ord, PartialOrd, Eq, IntoStaticStr,
)]
#[strum(serialize_all = "kebab-case")]
#[serde(rename_all = "kebab-case")]
#[strum_discriminants(name(AttributeDiscriminant))]
#[strum_discriminants(derive(EnumString))]
#[strum_discriminants(strum(serialize_all = "kebab-case"))]
pub(crate) enum Attribute<'src> {
  Confirm(Option<StringLiteral<'src>>),
  Doc(Option<StringLiteral<'src>>),
  Group(StringLiteral<'src>),
  Linux,
  Macos,
  NoCd,
  NoExitMessage,
  NoQuiet,
  PositionalArguments,
  Private,
  Unix,
  Windows,
}

impl AttributeDiscriminant {
  fn argument_range(self) -> RangeInclusive<usize> {
    match self {
      Self::Confirm | Self::Doc => 0..=1,
      Self::Group => 1..=1,
      Self::Linux
      | Self::Macos
      | Self::NoCd
      | Self::NoExitMessage
      | Self::NoQuiet
      | Self::PositionalArguments
      | Self::Private
      | Self::Unix
      | Self::Windows => 0..=0,
    }
  }
}

impl<'src> Attribute<'src> {
  pub(crate) fn new(
    name: Name<'src>,
    argument: Option<StringLiteral<'src>>,
  ) -> CompileResult<'src, Self> {
    use AttributeDiscriminant::*;

    let discriminant = name
      .lexeme()
      .parse::<AttributeDiscriminant>()
      .ok()
      .ok_or_else(|| {
        name.error(CompileErrorKind::UnknownAttribute {
          attribute: name.lexeme(),
        })
      })?;

    let found = argument.as_ref().iter().count();
    let range = discriminant.argument_range();
    if !range.contains(&found) {
      return Err(
        name.error(CompileErrorKind::AttributeArgumentCountMismatch {
          attribute: name.lexeme(),
          found,
          min: *range.start(),
          max: *range.end(),
        }),
      );
    }

    Ok(match discriminant {
      Confirm => Self::Confirm(argument),
      Doc => Self::Doc(argument),
      Group => Self::Group(argument.unwrap()),
      Linux => Self::Linux,
      Macos => Self::Macos,
      NoCd => Self::NoCd,
      NoExitMessage => Self::NoExitMessage,
      NoQuiet => Self::NoQuiet,
      PositionalArguments => Self::PositionalArguments,
      Private => Self::Private,
      Unix => Self::Unix,
      Windows => Self::Windows,
    })
  }

  pub(crate) fn name(&self) -> &'static str {
    self.into()
  }

  fn argument(&self) -> Option<&StringLiteral> {
    match self {
      Self::Confirm(prompt) => prompt.as_ref(),
      Self::Doc(doc) => doc.as_ref(),
      Self::Group(group) => Some(group),
      Self::Linux
      | Self::Macos
      | Self::NoCd
      | Self::NoExitMessage
      | Self::NoQuiet
      | Self::PositionalArguments
      | Self::Private
      | Self::Unix
      | Self::Windows => None,
    }
  }
}

impl<'src> Display for Attribute<'src> {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(f, "{}", self.name())?;
    if let Some(argument) = self.argument() {
      write!(f, "({argument})")?;
    }

    Ok(())
  }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct AttributeSet<'src> {
  #[serde(flatten)]
  pub(crate) inner: BTreeSet<Attribute<'src>>,
}

impl<'src> AttributeSet<'src> {
  pub(crate) fn empty() -> Self {
    Self {
      inner: BTreeSet::new(),
    }
  }

  pub(crate) fn from_map<T>(input: BTreeMap<Attribute<'src>, T>) -> Self {
    Self {
      inner: input.into_keys().collect(),
    }
  }

  pub(crate) fn to_btree_set(self) -> BTreeSet<Attribute<'src>> {
    self.inner
  }

  pub(crate) fn contains(&self, attribute: &Attribute) -> bool {
    self.inner.contains(attribute)
  }

  /// Get the names of all Group attributes defined in this attribute set
  pub(crate) fn groups(&self) -> Vec<&StringLiteral<'src>> {
    self
      .inner
      .iter()
      .filter_map(|attr| {
        if let Attribute::Group(name) = attr {
          Some(name)
        } else {
          None
        }
      })
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn name() {
    assert_eq!(Attribute::NoExitMessage.name(), "no-exit-message");
  }
}
