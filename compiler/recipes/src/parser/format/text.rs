use std::collections::BTreeMap;

use serde::de::MapAccess;
use serde_yaml::Value;

use crate::ir::{FormatCommon, FormatInner, FormatText};

pub(super) fn text<'de, A>(
  _keys: BTreeMap<String, Value>,
  _map: A,
) -> Result<FormatInner, A::Error>
where
  A: MapAccess<'de>,
{
  Ok(FormatInner::Text(FormatText {
    common: Some(FormatCommon::default()),
  }))
}
