/*     _              _ _
 *  __| |_ _ ___ _ __( |_)_ _
 * / _` | '_/ _ \ '_ \/| | ' \
 * \__,_|_| \___/ .__/ |_|_||_| drop'in © 2019-2022 Blue Forest
 *              |_|
 * This code is free software distributed under GPLv3.
 */

use dropin::parser::read_values;
use dropin::types::TEXT;

#[test]
fn hello_world() {
  let value = read_values(vec![
    "values blueforest:tests:v1:hello_world",
    "======================================",
    "format text",
    "data \"hello world\"",
  ].join("\n"));
  let mut buf = Vec::new();
  value.compile(&mut buf);
  let mut ground_truth = vec![
    TEXT,
    0, // refs (options, format, flag...) length
  ];
  ground_truth.push(11); // hello world length
  ground_truth.extend("hello world".as_bytes());
  assert_eq!(buf, ground_truth);
}
