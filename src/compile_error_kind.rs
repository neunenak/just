use super::*;

#[derive(Debug, PartialEq)]
pub(crate) enum CompileErrorKind<'src> {
  AliasInvalidAttribute {
    alias: &'src str,
    attr: Attribute,
  },
  AliasShadowsRecipe {
    alias: &'src str,
    recipe_line: usize,
  },
  BacktickShebang,
  CircularRecipeDependency {
    recipe: &'src str,
    circle: Vec<&'src str>,
  },
  CircularVariableDependency {
    variable: &'src str,
    circle: Vec<&'src str>,
  },
  DependencyArgumentCountMismatch {
    dependency: &'src str,
    found: usize,
    min: usize,
    max: usize,
  },
  Redefinition {
    first: usize,
    first_type: &'static str,
    name: &'src str,
    second_type: &'static str,
  },
  DuplicateAttribute {
    attribute: &'src str,
    first: usize,
  },
  DuplicateParameter {
    recipe: &'src str,
    parameter: &'src str,
  },
  DuplicateSet {
    setting: &'src str,
    first: usize,
  },
  DuplicateVariable {
    variable: &'src str,
  },
  ExpectedKeyword {
    expected: Vec<Keyword>,
    found: Token<'src>,
  },
  ExtraLeadingWhitespace,
  FunctionArgumentCountMismatch {
    function: &'src str,
    found: usize,
    expected: Range<usize>,
  },
  Include,
  InconsistentLeadingWhitespace {
    expected: &'src str,
    found: &'src str,
  },
  Internal {
    message: String,
  },
  InvalidAttributeArgument {
    name: String,
    expected: bool,
  },
  InvalidEscapeSequence {
    character: char,
  },
  MismatchedClosingDelimiter {
    close: Delimiter,
    open: Delimiter,
    open_line: usize,
  },
  MixedLeadingWhitespace {
    whitespace: &'src str,
  },
  ParameterFollowsVariadicParameter {
    parameter: &'src str,
  },
  ParsingRecursionDepthExceeded,
  RequiredParameterFollowsDefaultParameter {
    parameter: &'src str,
  },
  UndefinedVariable {
    variable: &'src str,
  },
  UnexpectedCharacter {
    expected: char,
  },
  UnexpectedClosingDelimiter {
    close: Delimiter,
  },
  UnexpectedEndOfToken {
    expected: char,
  },
  UnexpectedToken {
    expected: Vec<TokenKind>,
    found: TokenKind,
  },
  UnknownAliasTarget {
    alias: &'src str,
    target: &'src str,
  },
  UnknownAttribute {
    attribute: &'src str,
  },
  UnknownDependency {
    recipe: &'src str,
    unknown: &'src str,
  },
  UnknownFunction {
    function: &'src str,
  },
  UnknownSetting {
    setting: &'src str,
  },
  UnknownStartOfToken,
  UnpairedCarriageReturn,
  UnterminatedBacktick,
  UnterminatedInterpolation,
  UnterminatedString,
}
