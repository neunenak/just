use super::*;

#[derive(Debug)]
pub(crate) struct Scope<'src: 'run, 'run> {
  parent: Option<&'run Self>,
  bindings: BTreeMap<&'src str, Binding<'src, String>>,
}

impl<'src, 'run> Scope<'src, 'run> {
  pub(crate) fn child(&'run self) -> Self {
    Self {
      parent: Some(self),
      bindings: BTreeMap::new(),
    }
  }

  pub(crate) fn new() -> Self {
    Self {
      parent: None,
      bindings: BTreeMap::new(),
    }
  }

  pub(crate) fn bind(&mut self, export: bool, name: Name<'src>, value: String) {
    let binding =Binding {
      depth: 0,
      export,
      name,
      value,
    };
    self.bindings.insert(binding.name(), binding);
  }

  pub(crate) fn bound(&self, name: &str) -> bool {
    self.bindings.contains_key(name)
  }

  pub(crate) fn value(&self, name: &str) -> Option<&str> {
    if let Some(binding) = self.bindings.get(name) {
      Some(binding.value.as_ref())
    } else {
      self.parent?.value(name)
    }
  }

  pub(crate) fn bindings(&self) -> impl Iterator<Item = &Binding<String>> {
    self.bindings.values()
  }

  pub(crate) fn names(&self) -> impl Iterator<Item = &str> {
    self.bindings.keys().copied()
  }

  pub(crate) fn parent(&self) -> Option<&'run Self> {
    self.parent
  }
}
