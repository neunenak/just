use super::*;

const WIDTH_LIMIT: usize = 30;

pub(crate) fn list_recipes_expanded(config: &Config, justfile: &Justfile) {
  let recipe_aliases = build_alias_table(&justfile);
  let line_widths = build_line_widths_table(&justfile, &recipe_aliases);
  let max_line_width = cmp::min(
    line_widths.values().copied().max().unwrap_or(0),
    WIDTH_LIMIT,
  );

  let doc_color = config.color.stdout().doc();
  print!("{}", config.list_heading);

  for recipe in justfile.public_recipes(config.unsorted) {
    let name = recipe.name();

    for (i, name) in iter::once(&name)
      .chain(recipe_aliases.get(name).unwrap_or(&Vec::new()))
      .enumerate()
    {
      print!("{}{name}", config.list_prefix);
      for parameter in &recipe.parameters {
        print!(" {}", parameter.color_display(config.color.stdout()));
      }

      // Declaring this outside of the nested loops will probably be more efficient,
      // but it creates all sorts of lifetime issues with variables inside the loops.
      // If this is inlined like the docs say, it shouldn't make any difference.
      let print_doc = |doc| {
        print!(
          " {:padding$}{} {}",
          "",
          doc_color.paint("#"),
          doc_color.paint(doc),
          padding =
            max_line_width.saturating_sub(line_widths.get(name).copied().unwrap_or(max_line_width))
        );
      };

      match (i, recipe.doc) {
        (0, Some(doc)) => print_doc(doc),
        (0, None) => (),
        _ => {
          let alias_doc = format!("alias for `{}`", recipe.name);
          print_doc(&alias_doc);
        }
      }
      println!();
    }
  }
}

fn build_alias_table<'a>(justfile: &'a Justfile) -> BTreeMap<&'a str, Vec<&'a str>> {
  let mut recipe_aliases: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
  for alias in justfile.aliases.values() {
    if alias.is_private() {
      continue;
    }

    let target_name = alias.target.name.lexeme();
    let alias_name = alias.name.lexeme();
    recipe_aliases
      .entry(target_name)
      .and_modify(|e| e.push(alias_name))
      .or_insert_with(|| vec![alias_name]);
  }
  recipe_aliases
}

fn build_line_widths_table<'a>(
  justfile: &'a Justfile,
  alias_table: &'a BTreeMap<&'a str, Vec<&'a str>>,
) -> BTreeMap<&'a str, usize> {
  let mut line_widths: BTreeMap<&str, usize> = BTreeMap::new();

  for (name, recipe) in &justfile.recipes {
    if recipe.private {
      continue;
    }

    for name in iter::once(name).chain(alias_table.get(name).unwrap_or(&Vec::new())) {
      let mut line_width = UnicodeWidthStr::width(*name);

      for parameter in &recipe.parameters {
        line_width +=
          UnicodeWidthStr::width(format!(" {}", parameter.color_display(Color::never())).as_str());
      }

      if line_width <= WIDTH_LIMIT {
        line_widths.insert(name, line_width);
      }
    }
  }

  line_widths
}
