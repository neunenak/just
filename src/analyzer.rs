use {super::*, CompileErrorKind::*};

#[derive(Default)]
pub(crate) struct Analyzer<'src> {
  assignments: Table<'src, Assignment<'src>>,
  aliases: Table<'src, Alias<'src, Name<'src>>>,
  sets: Table<'src, Set<'src>>,
}

impl<'src> Analyzer<'src> {
  pub(crate) fn analyze(
    asts: &HashMap<PathBuf, Ast<'src>>,
    doc: Option<&'src str>,
    loaded: &[PathBuf],
    name: Option<Name<'src>>,
    paths: &HashMap<PathBuf, PathBuf>,
    root: &Path,
  ) -> CompileResult<'src, Justfile<'src>> {
    Self::default().justfile(asts, doc, loaded, name, paths, root)
  }

  fn justfile(
    mut self,
    asts: &HashMap<PathBuf, Ast<'src>>,
    doc: Option<&'src str>,
    loaded: &[PathBuf],
    name: Option<Name<'src>>,
    paths: &HashMap<PathBuf, PathBuf>,
    root: &Path,
  ) -> CompileResult<'src, Justfile<'src>> {
    let mut recipes = Vec::new();

    let mut assignments = Vec::new();

    let mut stack = Vec::new();
    stack.push(asts.get(root).unwrap());

    let mut warnings = Vec::new();

    let mut modules: Table<Justfile> = Table::new();

    let mut unexports: HashSet<String> = HashSet::new();

    let mut definitions: HashMap<&str, (&'static str, Name)> = HashMap::new();

    let mut define = |name: Name<'src>,
                      second_type: &'static str,
                      duplicates_allowed: bool|
     -> CompileResult<'src> {
      if let Some((first_type, original)) = definitions.get(name.lexeme()) {
        if !(*first_type == second_type && duplicates_allowed) {
          let (original, redefinition) = if name.line < original.line {
            (name, *original)
          } else {
            (*original, name)
          };

          return Err(redefinition.token.error(Redefinition {
            first_type,
            second_type,
            name: name.lexeme(),
            first: original.line,
          }));
        }
      }

      definitions.insert(name.lexeme(), (second_type, name));

      Ok(())
    };

    while let Some(ast) = stack.pop() {
      for item in &ast.items {
        match item {
          Item::Alias(alias) => {
            define(alias.name, "alias", false)?;
            Self::analyze_alias(alias)?;
            self.aliases.insert(alias.clone());
          }
          Item::Assignment(assignment) => {
            assignments.push(assignment);
          }
          Item::Comment(_) => (),
          Item::Import {
            absolute,
            attributes,
            ..
          } => {
            //TODO check attributes for validity

            let _groups = attributes.groups();

            if let Some(absolute) = absolute {
              stack.push(asts.get(absolute).unwrap());
            }
          }
          Item::Module {
            absolute,
            name,
            doc,
            ..
          } => {
            if let Some(absolute) = absolute {
              define(*name, "module", false)?;
              modules.insert(Self::analyze(
                asts,
                *doc,
                loaded,
                Some(*name),
                paths,
                absolute,
              )?);
            }
          }
          Item::Recipe(recipe) => {
            if recipe.enabled() {
              Self::analyze_recipe(recipe)?;
              recipes.push(recipe);
            }
          }
          Item::Set(set) => {
            self.analyze_set(set)?;
            self.sets.insert(set.clone());
          }
          Item::Unexport { name } => {
            if !unexports.insert(name.lexeme().to_string()) {
              return Err(name.token.error(DuplicateUnexport {
                variable: name.lexeme(),
              }));
            }
          }
        }
      }

      warnings.extend(ast.warnings.iter().cloned());
    }

    let settings = Settings::from_setting_iter(self.sets.into_iter().map(|(_, set)| set.value));

    let mut recipe_table: Table<'src, UnresolvedRecipe<'src>> = Table::default();

    for assignment in assignments {
      let variable = assignment.name.lexeme();

      if !settings.allow_duplicate_variables && self.assignments.contains_key(variable) {
        return Err(assignment.name.token.error(DuplicateVariable { variable }));
      }

      if self
        .assignments
        .get(variable)
        .map_or(true, |original| assignment.depth <= original.depth)
      {
        self.assignments.insert(assignment.clone());
      }

      if unexports.contains(variable) {
        return Err(assignment.name.token.error(ExportUnexported { variable }));
      }
    }

    AssignmentResolver::resolve_assignments(&self.assignments)?;

    for recipe in recipes {
      define(recipe.name, "recipe", settings.allow_duplicate_recipes)?;
      if recipe_table
        .get(recipe.name.lexeme())
        .map_or(true, |original| recipe.file_depth <= original.file_depth)
      {
        recipe_table.insert(recipe.clone());
      }
    }

    let recipes = RecipeResolver::resolve_recipes(&self.assignments, &settings, recipe_table)?;

    let mut aliases = Table::new();
    while let Some(alias) = self.aliases.pop() {
      aliases.insert(Self::resolve_alias(&recipes, alias)?);
    }

    let root = paths.get(root).unwrap();

    Ok(Justfile {
      aliases,
      assignments: self.assignments,
      default: recipes
        .values()
        .filter(|recipe| recipe.name.path == root)
        .fold(None, |accumulator, next| match accumulator {
          None => Some(Rc::clone(next)),
          Some(previous) => Some(if previous.line_number() < next.line_number() {
            previous
          } else {
            Rc::clone(next)
          }),
        }),
      doc,
      loaded: loaded.into(),
      modules,
      name,
      recipes,
      settings,
      source: root.into(),
      unexports,
      warnings,
    })
  }

  fn analyze_recipe(recipe: &UnresolvedRecipe<'src>) -> CompileResult<'src> {
    let mut parameters = BTreeSet::new();
    let mut passed_default = false;

    for parameter in &recipe.parameters {
      if parameters.contains(parameter.name.lexeme()) {
        return Err(parameter.name.token.error(DuplicateParameter {
          recipe: recipe.name.lexeme(),
          parameter: parameter.name.lexeme(),
        }));
      }
      parameters.insert(parameter.name.lexeme());

      if parameter.default.is_some() {
        passed_default = true;
      } else if passed_default {
        return Err(
          parameter
            .name
            .token
            .error(RequiredParameterFollowsDefaultParameter {
              parameter: parameter.name.lexeme(),
            }),
        );
      }
    }

    let mut continued = false;
    for line in &recipe.body {
      if !recipe.shebang && !continued {
        if let Some(Fragment::Text { token }) = line.fragments.first() {
          let text = token.lexeme();

          if text.starts_with(' ') || text.starts_with('\t') {
            return Err(token.error(ExtraLeadingWhitespace));
          }
        }
      }

      continued = line.is_continuation();
    }

    Ok(())
  }

  fn analyze_alias(alias: &Alias<'src, Name<'src>>) -> CompileResult<'src> {
    let name = alias.name.lexeme();

    for attribute in &alias.attributes.inner {
      if *attribute != Attribute::Private {
        return Err(alias.name.token.error(AliasInvalidAttribute {
          alias: name,
          attribute: attribute.clone(),
        }));
      }
    }

    Ok(())
  }

  fn analyze_set(&self, set: &Set<'src>) -> CompileResult<'src> {
    if let Some(original) = self.sets.get(set.name.lexeme()) {
      return Err(set.name.error(DuplicateSet {
        setting: original.name.lexeme(),
        first: original.name.line,
      }));
    }

    Ok(())
  }

  fn resolve_alias(
    recipes: &Table<'src, Rc<Recipe<'src>>>,
    alias: Alias<'src, Name<'src>>,
  ) -> CompileResult<'src, Alias<'src>> {
    // Make sure the alias doesn't conflict with any recipe
    if let Some(recipe) = recipes.get(alias.name.lexeme()) {
      return Err(alias.name.token.error(AliasShadowsRecipe {
        alias: alias.name.lexeme(),
        recipe_line: recipe.line_number(),
      }));
    }

    // Make sure the target recipe exists
    match recipes.get(alias.target.lexeme()) {
      Some(target) => Ok(alias.resolve(Rc::clone(target))),
      None => Err(alias.name.token.error(UnknownAliasTarget {
        alias: alias.name.lexeme(),
        target: alias.target.lexeme(),
      })),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  analysis_error! {
    name: duplicate_alias,
    input: "alias foo := bar\nalias foo := baz",
    offset: 23,
    line: 1,
    column: 6,
    width: 3,
    kind: Redefinition { first_type: "alias", second_type: "alias", name: "foo", first: 0 },
  }

  analysis_error! {
    name: unknown_alias_target,
    input: "alias foo := bar\n",
    offset: 6,
    line: 0,
    column: 6,
    width: 3,
    kind: UnknownAliasTarget {alias: "foo", target: "bar"},
  }

  analysis_error! {
    name: alias_shadows_recipe_before,
    input: "bar: \n  echo bar\nalias foo := bar\nfoo:\n  echo foo",
    offset: 34,
    line: 3,
    column: 0,
    width: 3,
    kind: Redefinition { first_type: "alias", second_type: "recipe", name: "foo", first: 2 },
  }

  analysis_error! {
    name: alias_shadows_recipe_after,
    input: "foo:\n  echo foo\nalias foo := bar\nbar:\n  echo bar",
    offset: 22,
    line: 2,
    column: 6,
    width: 3,
    kind: Redefinition { first_type: "alias", second_type: "recipe", name: "foo", first: 0 },
  }

  analysis_error! {
    name:   required_after_default,
    input:  "hello arg='foo' bar:",
    offset:  16,
    line:   0,
    column: 16,
    width:  3,
    kind:   RequiredParameterFollowsDefaultParameter{parameter: "bar"},
  }

  analysis_error! {
    name:   duplicate_parameter,
    input:  "a b b:",
    offset:  4,
    line:   0,
    column: 4,
    width:  1,
    kind:   DuplicateParameter{recipe: "a", parameter: "b"},
  }

  analysis_error! {
    name:   duplicate_variadic_parameter,
    input:  "a b +b:",
    offset: 5,
    line:   0,
    column: 5,
    width:  1,
    kind:   DuplicateParameter{recipe: "a", parameter: "b"},
  }

  analysis_error! {
    name:   duplicate_recipe,
    input:  "a:\nb:\na:",
    offset:  6,
    line:   2,
    column: 0,
    width:  1,
    kind:   Redefinition { first_type: "recipe", second_type: "recipe", name: "a", first: 0 },
  }

  analysis_error! {
    name:   duplicate_variable,
    input:  "a := \"0\"\na := \"0\"",
    offset: 9,
    line:   1,
    column: 0,
    width:  1,
    kind:   DuplicateVariable{variable: "a"},
  }

  analysis_error! {
    name:   extra_whitespace,
    input:  "a:\n blah\n  blarg",
    offset:  10,
    line:   2,
    column: 1,
    width:  6,
    kind:   ExtraLeadingWhitespace,
  }
}
