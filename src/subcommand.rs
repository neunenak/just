use super::*;

const INIT_JUSTFILE: &str = "default:\n    echo 'Hello, world!'\n";

#[derive(PartialEq, Clone, Debug)]
pub(crate) enum Subcommand {
  Changelog,
  Choose {
    overrides: BTreeMap<String, String>,
    chooser: Option<String>,
  },
  Command {
    arguments: Vec<OsString>,
    binary: OsString,
    overrides: BTreeMap<String, String>,
  },
  Completions {
    shell: String,
  },
  Dump,
  Edit,
  Evaluate {
    overrides: BTreeMap<String, String>,
    variable: Option<String>,
  },
  Format,
  Init,
  List,
  Run {
    arguments: Vec<String>,
    overrides: BTreeMap<String, String>,
  },
  Show {
    name: String,
  },
  Summary,
  Variables,
}

impl Subcommand {
  pub(crate) fn execute<'src>(
    &self,
    config: &Config,
    loader: &'src Loader,
  ) -> Result<(), Error<'src>> {
    use Subcommand::*;

    match self {
      Changelog => {
        Self::changelog();
        return Ok(());
      }
      Completions { shell } => return Self::completions(shell),
      Init => return Self::init(config),
      Run {
        arguments,
        overrides,
      } => return Self::run(config, loader, arguments, overrides),
      _ => {}
    }

    let search = Search::find(&config.search_config, &config.invocation_directory)?;

    if let Edit = self {
      return Self::edit(&search);
    }

    let (src, ast, justfile) = Self::compile(config, loader, &search)?;

    match self {
      Choose { overrides, chooser } => {
        Self::choose(config, justfile, &search, overrides, chooser.as_deref())?;
      }
      Command { overrides, .. } | Evaluate { overrides, .. } => {
        justfile.run(config, &search, overrides, &[])?;
      }
      Dump => Self::dump(config, ast, justfile)?,
      Format => Self::format(config, &search, src, ast)?,
      List => Self::list(config, justfile),
      Show { ref name } => Self::show(config, name, justfile)?,
      Summary => Self::summary(config, justfile),
      Variables => Self::variables(justfile),
      Changelog | Completions { .. } | Edit | Init | Run { .. } => unreachable!(),
    }

    Ok(())
  }

  pub(crate) fn run<'src>(
    config: &Config,
    loader: &'src Loader,
    arguments: &[String],
    overrides: &BTreeMap<String, String>,
  ) -> Result<(), Error<'src>> {
    if matches!(
      config.search_config,
      SearchConfig::FromInvocationDirectory | SearchConfig::FromSearchDirectory { .. }
    ) {
      let starting_path = match &config.search_config {
        SearchConfig::FromInvocationDirectory => config.invocation_directory.clone(),
        SearchConfig::FromSearchDirectory { search_directory } => {
          env::current_dir().unwrap().join(search_directory)
        }
        _ => unreachable!(),
      };

      let mut path = starting_path.clone();

      let mut unknown_recipes_errors = None;

      loop {
        let search = match Search::find_next(&path) {
          Err(SearchError::NotFound) => match unknown_recipes_errors {
            Some(err) => return Err(err),
            None => return Err(SearchError::NotFound.into()),
          },
          Err(err) => return Err(err.into()),
          Ok(search) => {
            if config.verbosity.loquacious() && path != starting_path {
              eprintln!(
                "Trying {}",
                starting_path
                  .strip_prefix(path)
                  .unwrap()
                  .components()
                  .map(|_| path::Component::ParentDir)
                  .collect::<PathBuf>()
                  .join(search.justfile.file_name().unwrap())
                  .display()
              );
            }
            search
          }
        };

        match Self::run_inner(config, loader, arguments, overrides, &search) {
          Err((err @ Error::UnknownRecipes { .. }, true)) => {
            match search.justfile.parent().unwrap().parent() {
              Some(parent) => {
                unknown_recipes_errors.get_or_insert(err);
                path = parent.into();
              }
              None => return Err(err),
            }
          }
          result => return result.map_err(|(err, _fallback)| err),
        }
      }
    } else {
      Self::run_inner(
        config,
        loader,
        arguments,
        overrides,
        &Search::find(&config.search_config, &config.invocation_directory)?,
      )
      .map_err(|(err, _fallback)| err)
    }
  }

  fn run_inner<'src>(
    config: &Config,
    loader: &'src Loader,
    arguments: &[String],
    overrides: &BTreeMap<String, String>,
    search: &Search,
  ) -> Result<(), (Error<'src>, bool)> {
    let (_src, _ast, justfile) =
      Self::compile(config, loader, search).map_err(|err| (err, false))?;
    justfile
      .run(config, search, overrides, arguments)
      .map_err(|err| (err, justfile.settings.fallback))
  }

  fn compile<'src>(
    config: &Config,
    loader: &'src Loader,
    search: &Search,
  ) -> Result<(&'src str, Ast<'src>, Justfile<'src>), Error<'src>> {
    let src = loader.load(&search.justfile)?;

    let (ast, justfile) = Compiler::compile(src)?;

    if config.verbosity.loud() {
      for warning in &justfile.warnings {
        eprintln!("{}", warning.color_display(config.color.stderr()));
      }
    }

    Ok((src, ast, justfile))
  }

  fn changelog() {
    print!("{}", include_str!("../CHANGELOG.md"));
  }

  fn choose<'src>(
    config: &Config,
    justfile: Justfile<'src>,
    search: &Search,
    overrides: &BTreeMap<String, String>,
    chooser: Option<&str>,
  ) -> Result<(), Error<'src>> {
    let recipes = justfile
      .public_recipes(config.unsorted)
      .iter()
      .filter(|recipe| recipe.min_arguments() == 0)
      .copied()
      .collect::<Vec<&Recipe<Dependency>>>();

    if recipes.is_empty() {
      return Err(Error::NoChoosableRecipes);
    }

    let chooser = chooser
      .map(OsString::from)
      .or_else(|| env::var_os(config::CHOOSER_ENVIRONMENT_KEY))
      .unwrap_or_else(|| OsString::from(config::CHOOSER_DEFAULT));

    let result = justfile
      .settings
      .shell_command(config)
      .arg(&chooser)
      .current_dir(&search.working_directory)
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .spawn();

    let mut child = match result {
      Ok(child) => child,
      Err(io_error) => {
        let (shell_binary, shell_arguments) = justfile.settings.shell(config);
        return Err(Error::ChooserInvoke {
          shell_binary: shell_binary.to_owned(),
          shell_arguments: shell_arguments.join(" "),
          chooser,
          io_error,
        });
      }
    };

    for recipe in recipes {
      if let Err(io_error) = child
        .stdin
        .as_mut()
        .expect("Child was created with piped stdio")
        .write_all(format!("{}\n", recipe.name).as_bytes())
      {
        return Err(Error::ChooserWrite { io_error, chooser });
      }
    }

    let output = match child.wait_with_output() {
      Ok(output) => output,
      Err(io_error) => {
        return Err(Error::ChooserRead { io_error, chooser });
      }
    };

    if !output.status.success() {
      return Err(Error::ChooserStatus {
        status: output.status,
        chooser,
      });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    let recipes = stdout
      .split_whitespace()
      .map(str::to_owned)
      .collect::<Vec<String>>();

    justfile.run(config, search, overrides, &recipes)
  }

  fn completions(shell: &str) -> RunResult<'static, ()> {
    use clap::Shell;

    fn replace(haystack: &mut String, needle: &str, replacement: &str) -> RunResult<'static, ()> {
      if let Some(index) = haystack.find(needle) {
        haystack.replace_range(index..index + needle.len(), replacement);
        Ok(())
      } else {
        Err(Error::internal(format!(
          "Failed to find text:\n{needle}\n…in completion script:\n{haystack}"
        )))
      }
    }

    let shell = shell
      .parse::<Shell>()
      .expect("Invalid value for clap::Shell");

    let buffer = Vec::new();
    let mut cursor = Cursor::new(buffer);
    Config::app().gen_completions_to(env!("CARGO_PKG_NAME"), shell, &mut cursor);
    let buffer = cursor.into_inner();
    let mut script = String::from_utf8(buffer).expect("Clap completion not UTF-8");

    match shell {
      Shell::Bash => {
        for (needle, replacement) in completions::BASH_COMPLETION_REPLACEMENTS {
          replace(&mut script, needle, replacement)?;
        }
      }
      Shell::Fish => {
        script.insert_str(0, completions::FISH_RECIPE_COMPLETIONS);
      }
      Shell::PowerShell => {
        for (needle, replacement) in completions::POWERSHELL_COMPLETION_REPLACEMENTS {
          replace(&mut script, needle, replacement)?;
        }
      }

      Shell::Zsh => {
        for (needle, replacement) in completions::ZSH_COMPLETION_REPLACEMENTS {
          replace(&mut script, needle, replacement)?;
        }
      }
      Shell::Elvish => {}
    }

    println!("{}", script.trim());

    Ok(())
  }

  fn dump(config: &Config, ast: Ast, justfile: Justfile) -> Result<(), Error<'static>> {
    match config.dump_format {
      DumpFormat::Json => {
        serde_json::to_writer(io::stdout(), &justfile)
          .map_err(|serde_json_error| Error::DumpJson { serde_json_error })?;
        println!();
      }
      DumpFormat::Just => print!("{ast}"),
    }
    Ok(())
  }

  fn edit(search: &Search) -> Result<(), Error<'static>> {
    let editor = env::var_os("VISUAL")
      .or_else(|| env::var_os("EDITOR"))
      .unwrap_or_else(|| "vim".into());

    let error = Command::new(&editor)
      .current_dir(&search.working_directory)
      .arg(&search.justfile)
      .status();

    let status = match error {
      Err(io_error) => return Err(Error::EditorInvoke { editor, io_error }),
      Ok(status) => status,
    };

    if !status.success() {
      return Err(Error::EditorStatus { editor, status });
    }

    Ok(())
  }

  fn format(config: &Config, search: &Search, src: &str, ast: Ast) -> Result<(), Error<'static>> {
    config.require_unstable("The `--fmt` command is currently unstable.")?;

    let formatted = ast.to_string();

    if config.check {
      return if formatted == src {
        Ok(())
      } else {
        if !config.verbosity.quiet() {
          use similar::{ChangeTag, TextDiff};

          let diff = TextDiff::configure()
            .algorithm(similar::Algorithm::Patience)
            .diff_lines(src, &formatted);

          for op in diff.ops() {
            for change in diff.iter_changes(op) {
              let (symbol, color) = match change.tag() {
                ChangeTag::Delete => ("-", config.color.stdout().diff_deleted()),
                ChangeTag::Equal => (" ", config.color.stdout()),
                ChangeTag::Insert => ("+", config.color.stdout().diff_added()),
              };

              print!("{}{symbol}{change}{}", color.prefix(), color.suffix());
            }
          }
        }

        Err(Error::FormatCheckFoundDiff)
      };
    }

    fs::write(&search.justfile, formatted).map_err(|io_error| Error::WriteJustfile {
      justfile: search.justfile.clone(),
      io_error,
    })?;

    if config.verbosity.loud() {
      eprintln!("Wrote justfile to `{}`", search.justfile.display());
    }

    Ok(())
  }

  fn init(config: &Config) -> Result<(), Error<'static>> {
    let search = Search::init(&config.search_config, &config.invocation_directory)?;

    if search.justfile.is_file() {
      Err(Error::InitExists {
        justfile: search.justfile,
      })
    } else if let Err(io_error) = fs::write(&search.justfile, INIT_JUSTFILE) {
      Err(Error::WriteJustfile {
        justfile: search.justfile,
        io_error,
      })
    } else {
      if config.verbosity.loud() {
        eprintln!("Wrote justfile to `{}`", search.justfile.display());
      }
      Ok(())
    }
  }

  fn list(config: &Config, justfile: Justfile) {
    crate::list_recipes::list_recipes_expanded(config, &justfile);
  }

  fn show<'src>(config: &Config, name: &str, justfile: Justfile<'src>) -> Result<(), Error<'src>> {
    if let Some(alias) = justfile.get_alias(name) {
      let recipe = justfile.get_recipe(alias.target.name.lexeme()).unwrap();
      println!("{alias}");
      println!("{}", recipe.color_display(config.color.stdout()));
      Ok(())
    } else if let Some(recipe) = justfile.get_recipe(name) {
      println!("{}", recipe.color_display(config.color.stdout()));
      Ok(())
    } else {
      Err(Error::UnknownRecipes {
        recipes: vec![name.to_owned()],
        suggestion: justfile.suggest_recipe(name),
      })
    }
  }

  fn summary(config: &Config, justfile: Justfile) {
    if justfile.count() == 0 {
      if config.verbosity.loud() {
        eprintln!("Justfile contains no recipes.");
      }
    } else {
      let summary = justfile
        .public_recipes(config.unsorted)
        .iter()
        .map(|recipe| recipe.name())
        .collect::<Vec<&str>>()
        .join(" ");
      println!("{summary}");
    }
  }

  fn variables(justfile: Justfile) {
    for (i, (_, assignment)) in justfile.assignments.iter().enumerate() {
      if i > 0 {
        print!(" ");
      }
      print!("{}", assignment.name);
    }
    println!();
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn init_justfile() {
    testing::compile(INIT_JUSTFILE);
  }
}
