//! Offline Mercenary parser, editable group models and complete Trade body comparison.
#[allow(dead_code)]
#[path = "../src/calculated.rs"]
mod calculated;
#[allow(dead_code)]
#[path = "../src/filters.rs"]
mod filters;
#[allow(dead_code)]
#[path = "../src/item.rs"]
mod item;
#[allow(dead_code)]
#[path = "../src/item_filters.rs"]
mod item_filters;
#[allow(dead_code)]
#[path = "../src/modifiers.rs"]
mod modifiers;
#[allow(dead_code)]
#[path = "../src/presets.rs"]
mod presets;
#[allow(dead_code)]
#[path = "../src/properties.rs"]
mod properties;
#[allow(dead_code)]
#[path = "../src/pseudo.rs"]
mod pseudo;
use serde::Deserialize;
#[derive(Deserialize)]
struct Input {
    text: String,
    #[serde(default)]
    edits: Vec<Edit>,
}
#[derive(Deserialize)]
struct Edit {
    node: usize,
    child: Option<usize>,
    enabled: Option<bool>,
    option: Option<u8>,
    min: Option<f64>,
    #[serde(default)]
    clear_min: bool,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Read;
    let mut input = String::new();
    std::io::stdin()
        .take(32 * 1024 * 1024)
        .read_to_string(&mut input)?;
    let cases: Vec<Input> = serde_json::from_str(&input)?;
    let output:Vec<_>=cases.into_iter().map(|i|{
  let item=item::Item::parse(&i.text)?;let parsed=item.mercenary.as_ref().ok_or("Missing mercenary")?;
  let mut preset=parsed.preset()?;let initial=preset.clone();
  for edit in i.edits {
   let node=preset.nodes.get_mut(edit.node).ok_or("Invalid node")?;
   let f=match node {item::mercenary::Node::Stat(f)=>f,item::mercenary::Node::Group(g)=>if let Some(index)=edit.child {g.stats.get_mut(index).ok_or("Invalid child")?}else{&mut g.meta}};
   if let Some(enabled)=edit.enabled {f.disabled = !enabled;}
   if let Some(option)=edit.option {f.option=Some(option);}
   if edit.clear_min {f.min=None;}else if let Some(min)=edit.min {f.min=Some(min);}
  }
  let plan=item_filters::create(&item,&item_filters::Options{exact:true,league:"Covens".into(),merchant_only:false,..Default::default()})?;
  let query=filters::query(&item,&filters::Filters{item:Some(plan.clone()),mercenary:Some(preset.clone()),..Default::default()})?;
  Ok::<_,String>(serde_json::json!({"parsed":parsed,"initial":initial,"edited":preset,"plan":plan,"query":query}))
 }).collect::<Result<_,_>>()?;
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}
