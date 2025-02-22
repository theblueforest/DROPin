use alloc::{
  fmt::{self, Write},
  string::String,
  vec::Vec,
};
use dropin_compiler_common::to_upper_camelcase;
use dropin_compiler_recipes::ir::{ComponentChildInner, ComponentZone};

use crate::{
  formats::FormatsState,
  gen::expressions::gen_rich_text,
  objects_getter::ObjectGetterState,
  properties_resolver::PropertiesResolverState,
  updated_listeners::{
    write_notifier_name, write_updater_name, UpdatedAndListenersState,
  },
  Stated,
};

use super::{
  expressions::{gen_expressions, gen_getter},
  formats::gen_format,
  Sub,
};

pub fn gen_zone<'a, S>(
  output: &mut String,
  component: &str,
  state: &S,
  trace: &[usize],
  zone: &ComponentZone,
) -> fmt::Result
where
  S: Sub<'a>,
{
  write!(output, "Row(children: [")?;
  let updated_listeners = <S as Stated<UpdatedAndListenersState>>::state(state);
  let notifiers = &updated_listeners.get_notifiers(component);
  for (i, child) in zone.blocks.iter().enumerate() {
    if i != 0 {
      write!(output, ",")?;
    }
    let trace = &[trace, &[i]].concat();
    let updated_listeners = updated_listeners
      .get_listeners(component, trace)
      .map(|listeners| {
        listeners
          .iter()
          .filter(|listener| {
            notifiers
              .iter()
              .position(|updated| updated.getter.as_ref() == listener.getter)
              .is_some()
          })
          .collect::<Vec<_>>()
      });
    let is_listenable = if let ComponentChildInner::Extern(_) =
      child.component_child_inner.as_ref().unwrap()
    {
      false
    } else {
      let mut is_listenable = false;
      if let Some(updated_listeners) = updated_listeners {
        if !updated_listeners.is_empty() {
          assert_eq!(
            updated_listeners.len(),
            1,
            "TODO: add Listenable.merge()"
          );
          is_listenable = true;
          let listener = &updated_listeners[0];
          write!(
            output,
            "ListenableBuilder(\
            listenable:",
          )?;
          write_notifier_name(output, listener.getter)?;
          write!(
            output,
            ", builder: (BuildContext context, Widget? child) => "
          )?;
        }
      }
      is_listenable
    };
    match child.component_child_inner.as_ref().unwrap() {
      ComponentChildInner::Text(text) => {
        write!(output, "Text(")?;
        gen_rich_text(
          output,
          component,
          state,
          &[],
          text.content.as_ref().unwrap(),
        )?;
        write!(output, ")")?;
      }
      ComponentChildInner::Input(input) => {
        write!(
          output,
          "SizedBox(width: 250, child: TextFormField(initialValue:"
        )?;
        gen_getter(
          output,
          component,
          state,
          input.on_change.as_ref().unwrap(),
        )?;
        write!(output, ", onChanged: widget.")?;
        write_updater_name(output, input.on_change.as_ref().unwrap())?;
        write!(output, "))")?;
      }
      ComponentChildInner::Extern(r#extern) => {
        write!(output, "{}(", to_upper_camelcase(&r#extern.id))?;
        let mut is_first = true;
        let objects = <S as Stated<ObjectGetterState>>::state(state);
        let resolver = <S as Stated<PropertiesResolverState>>::state(state);
        let formats = <S as Stated<FormatsState>>::state(state);
        for (key, value) in &r#extern.properties.as_ref().unwrap().values {
          if !is_first {
            write!(output, ",")?;
          }
          is_first = false;
          write!(output, "{key}:")?;
          gen_expressions(
            output,
            component,
            state,
            &[key.as_str()],
            false,
            value,
          )?;
          if objects.contains_object(&r#extern.id, &[key]) {
            write!(output, " as dynamic")?;
          }
        }
        for updated_getter in notifiers {
          if let Some(updated_by) =
            updated_getter.updated_by.get(r#extern.id.as_str())
          {
            write!(output, ",")?;
            write_notifier_name(output, updated_by)?;
            write!(output, ": widget.")?;
            write_notifier_name(output, &updated_getter.getter)?;
            write!(output, ",")?;
            write_updater_name(output, updated_by)?;
            write!(output, ":")?;
            if resolver
              .is_variable(component, updated_getter.getter.ident.as_str())
            {
              let format = formats
                .format_of(component, &updated_getter.getter)
                .unwrap();
              write!(output, "(")?;
              gen_format(output, state, &[], &format)?;
              write!(output, " new_) {{")?;
              gen_getter(output, component, state, &updated_getter.getter)?;
              write!(
                output,
                "= new_;\
                widget."
              )?;
              write_notifier_name(output, &updated_getter.getter)?;
              write!(output, ".notifyListeners();}}")?;
            } else {
              write!(output, "widget.")?;
              write_updater_name(output, &updated_getter.getter)?;
            }
          }
        }
        write!(output, ")")?;
      }
    }
    if is_listenable {
      write!(output, ")")?;
    }
  }
  write!(output, "])")?;
  Ok(())
}
