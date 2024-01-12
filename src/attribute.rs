use super::*;

#[derive(EnumString, PartialEq, Debug, Clone, Serialize, Ord, PartialOrd, Eq, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Attribute {
  Confirm,
  Linux,
  Macos,
  NoCd,
  NoExitMessage,
  Private,
  Unix,
  Windows,
  Group { name: String },
}

impl fmt::Display for Attribute {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      Attribute::Group { name } => write!(f, "group:{name}"),
      other => {
        let s: &str = other.into();
        write!(f, "{s}")
      }
    }
  }
}

impl Attribute {
  pub(crate) fn parse(
    name: &str,
    maybe_argument: Option<String>,
  ) -> Result<Attribute, CompileErrorKind> {
    match (name, maybe_argument) {
      ("group", Some(name)) => Ok(Attribute::Group { name }),
      ("group", None) => Err(CompileErrorKind::InvalidAttributeArgument {
        name: name.to_string(),
        expected: true,
      }),
      (other, None) => other
        .parse()
        .map_err(|_| CompileErrorKind::UnknownAttribute { attribute: name }),
      (_other, Some(_)) => Err(CompileErrorKind::InvalidAttributeArgument {
        name: name.to_string(),
        expected: false,
      }),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn to_str() {
    assert_eq!(Attribute::NoExitMessage.to_string(), "no-exit-message");
  }

  #[test]
  fn group() {
    assert_eq!(
      Attribute::Group {
        name: "linter".to_string()
      }
      .to_string(),
      "group:linter"
    );
  }
}
