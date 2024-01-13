use super::*;
const MAX_LINE_WIDTH: usize = 30;

pub(crate) fn list_groups(_config: &Config, justfile: &Justfile) {
  let mut group_names: Vec<&str> = justfile.public_groups().into_iter().collect();
  group_names.sort();
  for group in group_names {
    println!("{group}");
  }
}

fn get_recipe_aliases<'a>(justfile: &'a Justfile) -> BTreeMap<&'a str, Vec<&'a str>> {
  let mut recipe_aliases: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
  for alias in justfile.aliases.values() {
    if alias.is_private() {
      continue;
    }
    recipe_aliases
      .entry(alias.target.name.lexeme())
      .and_modify(|e| e.push(alias.name.lexeme()))
      .or_insert(vec![alias.name.lexeme()]);
  }
  recipe_aliases
}

fn get_line_widths<'a>(
  config: &Config,
  justfile: &'a Justfile,
  recipe_aliases: &BTreeMap<&'a str, Vec<&'a str>>,
) -> BTreeMap<&'a str, usize> {
  let mut line_widths: BTreeMap<&str, usize> = BTreeMap::new();

  for recipe in &justfile.public_recipes(config.unsorted) {
    let name = recipe.name.lexeme();

    for name in iter::once(&name).chain(recipe_aliases.get(name).unwrap_or(&Vec::new())) {
      let mut line_width = UnicodeWidthStr::width(*name);

      for parameter in &recipe.parameters {
        line_width +=
          UnicodeWidthStr::width(format!(" {}", parameter.color_display(Color::never())).as_str());
      }

      if line_width <= MAX_LINE_WIDTH {
        line_widths.insert(name, line_width);
      }
    }
  }

  line_widths
}

fn print_recipe(recipe: &Recipe, aliases: &Vec<&str>) {}

fn print_doc_comment(doc: &str, padding: usize, doc_color: Color) {
  print!(
    " {:padding$}{} {}",
    "",
    doc_color.paint("#"),
    doc_color.paint(doc),
    padding = padding
  );
}

pub(crate) fn list(config: &Config, level: usize, justfile: &Justfile) {
  if config.groups {
    print!("GROUPS palceholder");
    return;
  }

  let recipe_aliases = get_recipe_aliases(justfile);
  let line_widths = get_line_widths(config, justfile, &recipe_aliases);
  let max_line_width = cmp::min(
    line_widths.values().copied().max().unwrap_or(0),
    MAX_LINE_WIDTH,
  );
  let doc_color = config.color.stdout().doc();

  if level == 0 {
    print!("{}", config.list_heading);
  }

  for recipe in justfile.public_recipes(config.unsorted) {
    let name = recipe.name();

    for (i, name) in iter::once(&name)
      .chain(recipe_aliases.get(name).unwrap_or(&Vec::new()))
      .enumerate()
    {
      print!("{}{name}", config.list_prefix.repeat(level + 1));
      for parameter in &recipe.parameters {
        print!(" {}", parameter.color_display(config.color.stdout()));
      }

      let padding =
        max_line_width.saturating_sub(line_widths.get(name).copied().unwrap_or(max_line_width));
      match (i, recipe.doc) {
        (0, Some(doc)) => print_doc_comment(&doc, padding, doc_color),
        (0, None) => (),
        _ => {
          let alias_doc = format!("alias for `{}`", recipe.name);
          print_doc_comment(&alias_doc, padding, doc_color);
        }
      }
      println!();
    }
  }

  for (name, module) in &justfile.modules {
    println!("    {name}:");
    list(config, level + 1, module);
  }
}
