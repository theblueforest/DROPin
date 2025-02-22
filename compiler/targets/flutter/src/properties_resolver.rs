use core::ops::Deref;

use alloc::{
  borrow::Cow,
  collections::{BTreeMap, BTreeSet, VecDeque},
  vec::Vec,
};
use dropin_compiler_common::Key;
use dropin_compiler_recipes::ir::{
  Component, ComponentChild, ComponentChildInner, Expression, Getter,
};
use itertools::iproduct;

use crate::{visit::ExpressionTrace, Stated, Visit};

type PropertiesByComponent<'a> = BTreeMap<&'a str, PropertiesByProperty<'a>>;
type PropertiesByProperty<'a> =
  BTreeMap<&'a str, PropertiesByVariableOwner<'a>>;
type PropertiesByVariableOwner<'a> = BTreeMap<&'a str, Vec<Cow<'a, Getter>>>;

#[derive(Debug)]
pub struct PropertiesResolverState<'a> {
  component_variables: BTreeMap<&'a str, BTreeSet<&'a str>>,
  properties: PropertiesByComponent<'a>,
  pub redirections: PropertiesByComponent<'a>,
}

impl<'a> PropertiesResolverState<'a> {
  pub fn is_variable(&self, component: &str, ident: &str) -> bool {
    self.component_variables[component].contains(ident)
  }
}

impl<'a> Stated<PropertiesResolverState<'a>> for PropertiesResolverState<'a> {
  fn state(&self) -> &PropertiesResolverState<'a> {
    &self
  }
}

impl<'a> Deref for PropertiesResolverState<'a> {
  type Target = PropertiesByComponent<'a>;
  fn deref(&self) -> &Self::Target {
    &self.properties
  }
}

#[derive(Default)]
pub struct PropertiesResolver<'a> {
  component_id: Option<&'a str>,
  component_blocks: &'a [ComponentChild],
  component_variables: BTreeMap<&'a str, BTreeSet<&'a str>>,
  properties: PropertiesByComponent<'a>,
  redirections: PropertiesByComponent<'a>,
}

impl<'a> Visit<'a, PropertiesResolverState<'a>> for PropertiesResolver<'a> {
  fn build(mut self) -> PropertiesResolverState<'a> {
    let mut properties_to_insert = PropertiesByComponent::new();
    let mut redirections_to_insert = PropertiesByComponent::new();

    for (redirect_component, redirect_by_property) in &self.redirections {
      for (redirect_property, redirect_by_component) in redirect_by_property {
        for (redirect_owner, redirect_getters) in redirect_by_component {
          for redirect_getter in redirect_getters {
            let mut suffix =
              BTreeMap::<&str, BTreeMap<&str, VecDeque<Expression>>>::new();
            let mut owners = Vec::with_capacity(1);
            owners.push((
              *redirect_owner,
              redirect_getter.ident.as_str(),
              None,
            ));

            let mut all_props_by_property = Vec::with_capacity(1);
            while !owners.is_empty() {
              while let Some((owner, ident, child)) = owners.pop() {
                if let Some(child_suffix) =
                  child.and_then(|child| suffix.get(child))
                {
                  // 1
                  let child_suffix = child_suffix.clone();
                  let suffix = suffix.entry(owner).or_insert(BTreeMap::new());
                  for (prop, indexes) in child_suffix.into_iter().rev() {
                    let prop_suffix =
                      suffix.entry(prop).or_insert(VecDeque::new());
                    for index in indexes {
                      prop_suffix.push_back(index);
                    }
                  }
                }
                if let Some(props_by_property) = self.properties.get(owner) {
                  // 2
                  if let Some(suffix_child) =
                    suffix.get(owner).map(|child| (*child).clone())
                  {
                    for prop_by_owner in props_by_property.values() {
                      for prop_component in prop_by_owner.keys() {
                        let suffix_child = suffix_child.clone();
                        suffix.insert(prop_component, suffix_child);
                      }
                    }
                  }
                  all_props_by_property.push(props_by_property);
                  continue;
                }
                // 3
                let indirect =
                  self.redirections.get(owner).unwrap().get(ident).unwrap();
                for (new_owner, getters) in indirect {
                  for getter in getters {
                    let suffix = suffix
                      .entry(new_owner)
                      .or_insert(BTreeMap::new())
                      .entry(redirect_getter.ident.as_str())
                      .or_insert(VecDeque::new());
                    for index in getter.indexes.iter().rev() {
                      suffix.push_front(index.clone());
                    }
                    owners.push((
                      *new_owner,
                      getter.ident.as_str(),
                      Some(owner),
                    ));
                  }
                  redirections_to_insert
                    .entry(redirect_component)
                    .or_insert(PropertiesByProperty::new())
                    .entry(redirect_property)
                    .or_insert(PropertiesByVariableOwner::new())
                    .insert(*new_owner, getters.clone());
                }
              }
            }
            let suffix = suffix
              .iter_mut()
              .map(|(key, suffix)| {
                (
                  *key,
                  suffix
                    .iter_mut()
                    .map(|(key, suffix)| (*key, suffix.make_contiguous()))
                    .collect::<BTreeMap<_, _>>(),
                )
              })
              .collect::<BTreeMap<_, _>>();

            for props_by_property in &all_props_by_property {
              for props_by_owner in props_by_property.values() {
                for (prop_component, prop_getters) in props_by_owner {
                  let getters = iproduct!(prop_getters, redirect_getters)
                    .map(|(prop, redirect)| {
                      Cow::Owned(Getter {
                        ident: prop.ident.clone(),
                        indexes: [
                          prop.indexes.as_slice(),
                          suffix
                            .get(prop_component)
                            .and_then(|suffix| {
                              suffix.get(redirect_getter.ident.as_str())
                            })
                            .unwrap_or(&[].as_mut_slice()),
                          &redirect.indexes,
                        ]
                        .concat(),
                      })
                    })
                    .collect::<Vec<_>>();
                  properties_to_insert
                    .entry(redirect_component)
                    .or_insert(PropertiesByProperty::new())
                    .entry(redirect_property)
                    .or_insert(PropertiesByVariableOwner::new())
                    .entry(prop_component)
                    .and_modify(|current| current.extend_from_slice(&getters))
                    .or_insert(getters);
                }
              }
            }
          }
        }
      }
    }

    props_by_component_add_all(&mut self.properties, properties_to_insert);
    props_by_component_add_all(&mut self.redirections, redirections_to_insert);

    // todo!("{:#?}\n{:#?}", self.properties, self.redirections);

    PropertiesResolverState {
      component_variables: self.component_variables,
      properties: self.properties,
      redirections: self.redirections,
    }
  }

  fn visit_component(&mut self, component: &'a Component, _index: usize) {
    let mut component_variables = BTreeSet::new();
    if let Some(variables) = component.variables.as_ref() {
      for key_format in &variables.keys {
        component_variables.insert(key_format.key.as_str());
      }
    }
    self
      .component_variables
      .insert(&component.id, component_variables);
    self.component_id = Some(&component.id);
    self.component_blocks = &component.zone.as_ref().unwrap().blocks;
  }

  fn visit_getter(
    &mut self,
    getter: &'a Getter,
    mut trace: &ExpressionTrace<'a, '_>,
  ) {
    let mut key = None;
    loop {
      match &trace {
        ExpressionTrace::NestedQuantity {
          trace: parent,
          index,
          ..
        } => {
          key = Some(Key::Quantity(*index));
          trace = parent;
        }
        ExpressionTrace::NestedText {
          trace: parent,
          index,
          ..
        } => {
          key = Some(Key::Text(*index));
          trace = parent;
        }
        _ => break,
      }
    }
    let ExpressionTrace::ComponentChild(trace) = trace else {
      return;
    };
    let child = &self.component_blocks[trace.indexes[0]];
    // TODO: dig into zones
    let ComponentChildInner::Extern(r#extern) =
      child.component_child_inner.as_ref().unwrap()
    else {
      return;
    };
    let Some(Key::Text(property_key)) = key else {
      unreachable!();
    };
    let to_insert = if self.component_variables[self.component_id.unwrap()]
      .contains(getter.ident.as_str())
    {
      &mut self.properties
    } else {
      &mut self.redirections
    };
    to_insert
      .entry(&r#extern.id)
      .or_insert(PropertiesByProperty::new())
      .entry(property_key)
      .or_insert(PropertiesByVariableOwner::new())
      .entry(self.component_id.unwrap())
      .or_insert(Vec::with_capacity(1))
      .push(Cow::Borrowed(getter));
  }
}

fn props_by_component_add_all<'a>(
  destination: &mut PropertiesByComponent<'a>,
  other: PropertiesByComponent<'a>,
) {
  for (component, by_prop) in other {
    let entry = destination
      .entry(component)
      .or_insert(PropertiesByProperty::new());
    for (prop, by_owner) in by_prop {
      let entry = entry
        .entry(prop)
        .or_insert(PropertiesByVariableOwner::new());
      for (owner, getters) in by_owner {
        entry
          .entry(owner)
          .or_insert(Vec::with_capacity(getters.len()))
          .extend(getters);
      }
    }
  }
}
