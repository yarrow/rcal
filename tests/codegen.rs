use anyhow::{Context, Result, anyhow, bail};
use heck::{ToShoutyKebabCase, ToShoutySnakeCase};
use kdl::{KdlDocument, KdlNode, KdlValue, NodeKey};
use litemap::LiteMap;

const KDL: &str = include_str!("../assets/parameters.kdl");
const TEMPLATE: &str = include_str!("../assets/parameter_template.rs");

#[test]
#[cfg_attr(miri, ignore)] // Takes too long, and if we die generating code it will be obvious
fn codegen() {
    let mut content: Vec<u8> = vec![];
    generate(&mut content).unwrap();
    let content = String::from_utf8(content).unwrap();
    let content = codegenrs::rustfmt(content, None).unwrap();
    snapbox::assert_data_eq!(content, snapbox::file!["../src/parameter/parameter_value.rs"].raw());
}

fn generate<W: std::io::Write>(out: &mut W) -> Result<()> {
    let kdl: KdlDocument = KDL.parse()?;
    let param_info = param_info(&kdl)?;
    let type_info = type_info(&kdl)?;

    for line in TEMPLATE.lines() {
        if line.starts_with("    // ParameterValue") {
            type_info.write_param_values(&mut *out)?;
        } else if line.starts_with("// const") {
            write_constants(out, &param_info)?;
        } else if line.starts_with("    // Parameters") {
            for param in &param_info {
                param.write_methods(out, &type_info)?;
            }
        } else {
            writeln!(out, "{line}")?;
        }
    }
    Ok(())
}
trait GetStr {
    fn get(&self, key: impl Into<NodeKey>) -> Option<&KdlValue>;
    fn maybe_get_str(&self, key: &'static str) -> Result<Option<String>> {
        match self.get(key) {
            None => Ok(None),
            Some(KdlValue::String(s)) => Ok(Some(s.to_string())),
            Some(KdlValue::Integer(n)) => Ok(Some(format!("{n}"))),
            _ => {
                if key == "RFC" {
                    Err(anyhow!("The RFC value must be number"))
                } else {
                    Err(anyhow!("The {key} value must be an integer"))
                }
            }
        }
    }
    fn get_str(&self, key: &'static str) -> Result<String> {
        match self.maybe_get_str(key) {
            Ok(None) => Err(anyhow!("Expected {key} value")),
            Ok(Some(str)) => Ok(str),
            Err(e) => Err(e),
        }
    }
}
impl GetStr for KdlNode {
    fn get(&self, key: impl Into<NodeKey>) -> Option<&KdlValue> {
        KdlNode::get(self, key)
    }
}
#[derive(Debug)]
struct TypeInfo {
    variants: Vec<String>,
    type_of: LiteMap<String, String>,
    kind: LiteMap<String, String>,
}
impl TypeInfo {
    fn write_param_values<W: std::io::Write>(&self, mut out: W) -> Result<()> {
        for v in &self.variants {
            writeln!(out, "   {v}({}),", self.type_of(v)?)?;
        }
        Ok(())
    }
    fn is_copy(&self, variant: &str) -> bool {
        self.kind.get(variant).is_some()
    }
    fn is_single_valued(&self, variant: &str) -> bool {
        self.kind.get(variant).map(|s| s == "single_valued") == Some(true)
    }
    fn type_of(&self, variant: &str) -> Result<String> {
        Ok(self
            .type_of
            .get(variant)
            .with_context(|| format!("No type found for variant {variant}"))?
            .clone())
    }
}
fn type_info(kdl: &KdlDocument) -> Result<TypeInfo> {
    let mut variants = Vec::new();
    let mut type_of = LiteMap::new();
    let mut kind = LiteMap::new();
    let nodes = dash_nodes(kdl, "types")?;
    for node in nodes {
        let v = node.get_str("variant")?;
        variants.push(v.clone());
        let t = node.get_str("type")?;
        type_of.insert(v.clone(), t);
        if let Some(s) = node.get("kind") {
            match s.as_string() {
                Some(s) if s == "copy" || s == "single_valued" => {
                    kind.insert(v, s.to_string());
                }
                _ => panic!("kind must be either 'copy' or 'string'"),
            }
        }
    }
    Ok(TypeInfo { variants, type_of, kind })
}
#[derive(Debug)]
struct ParamInfo {
    rfc: String,
    section: String,
    method: String,
    variant: String,
    doc: Option<String>,
}
fn write_constants<W: std::io::Write>(out: &mut W, param_info: &Vec<ParamInfo>) -> Result<()> {
    for (n, info) in param_info.iter().enumerate() {
        writeln!(out, "   const {}: usize = {n};", info.konst())?;
    }
    writeln!(out, "pub(crate) const NAMES: [&str; {}] = [", param_info.len())?;
    for info in param_info {
        writeln!(out, "    {},", info.literal())?;
    }
    writeln!(out, "];")?;
    Ok(())
}
impl ParamInfo {
    fn konst(&self) -> String {
        self.method.to_shouty_snake_case()
    }
    fn literal(&self) -> String {
        format!(r#""{}""#, self.method.to_shouty_kebab_case())
    }
    fn write_methods<W: std::io::Write>(&self, out: &mut W, type_info: &TypeInfo) -> Result<()> {
        let link = {
            let (rfc, section) = (&self.rfc, &self.section);
            format!(
                "[RFC {rfc}, § {section}](https://datatracker.ietf.org/doc/html/rfc{rfc}#section-{section})"
            )
        };
        let method = self.method.clone();
        let konst = self.konst();
        let literal = self.literal();
        let variant = self.variant.clone();
        let typ = type_info.type_of(&variant)?;
        let (amphersand, star) = if type_info.is_copy(&variant) { ("", "*") } else { ("&", "") };

        writeln!(out, "/// Get the `{konst}` parameter ({link}).")?;
        if let Some(doc) = &self.doc {
            for docline in doc.lines() {
                writeln!(out, "/// {docline}")?;
            }
        }
        writeln!(
            out,
            r#"#[must_use]
            pub fn {method}(&self) -> Option<{amphersand}{typ}> {{
                match self.0.get(&{konst}) {{
                None => None,
                Some(ParameterValue::{variant}(value)) => Some({star}value),
                _ => panic!("Unexpected type for {{}}", {literal}),
                }}
            }}
            "#
        )?;
        writeln!(out, "/// Set the `{konst}` parameter ({link}).")?;
        if type_info.is_single_valued(&variant) {
            writeln!(
                out,
                "pub fn set_{method}(&mut self, value: Option<{typ}>) {{
                    match value {{
                        None => self.0.remove(&{konst}),
                        Some(v) => self.0.insert({konst}, ParameterValue::{variant}(v)),
                    }};
                }}"
            )?;
        } else {
            writeln!(
                out,
                "pub fn set_{method}(&mut self, value: {typ}) {{ 
                self.0.insert({konst}, ParameterValue::{variant}(value));
                }}"
            )?;
        }
        writeln!(out)?;
        Ok(())
    }
}

fn dash_nodes<'a>(kdl: &'a KdlDocument, name: &str) -> Result<&'a [KdlNode]> {
    let node = kdl.get(name).with_context(|| "Can't find {name} node")?;
    let Some(children) = node.children() else {
        bail!("{name} node is empty");
    };
    let nodes = children.nodes();
    for node in nodes {
        let name = node.name().value();
        if name != "-" {
            bail!(r#"Expected "-", found "{name}""#);
        }
    }
    Ok(nodes)
}
fn param_info(kdl: &KdlDocument) -> Result<Vec<ParamInfo>> {
    let nodes = dash_nodes(kdl, "parameters")?;
    let mut result = Vec::new();
    for node in nodes {
        let rfc = node.get_str("RFC")?;
        let section = node.get_str("Section")?;
        let method = node.get_str("method")?;
        let variant = node.get_str("variant")?;
        let doc = node.maybe_get_str("doc")?;
        result.push(ParamInfo { rfc, section, method, variant, doc });
    }
    Ok(result)
}
