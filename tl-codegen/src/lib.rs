extern crate pest;
#[macro_use]
extern crate pest_derive;
#[macro_use]
extern crate quote;

use pest::Parser;
use std::collections::HashMap;

#[derive(Parser)]
#[grammar = "tl.pest"]
struct TlParser;
const _GRAMMAR: &str = include_str!("tl.pest");

fn capitalize(s: &str) -> String {
    let mut v: Vec<char> = s.chars().collect();
    v[0] = v[0].to_uppercase().nth(0).unwrap();
    v.into_iter().collect()
}

fn convert_type(t: &str) -> String {
    match t {
        "double" => "f64".to_owned(),
        "string" => "String".to_owned(),
        "int32" => "i32".to_owned(),
        "int53" => "i64".to_owned(),
        "int64" => "i64".to_owned(),
        "Bool" => "bool".to_owned(),
        "bytes" => "String".to_owned(),
        _ => capitalize(t),
    }.to_owned()
}

fn convert_typeid(pair: pest::iterators::Pair<Rule>, res: &mut String) {
    match pair.as_rule() {
        Rule::vector => {
            let inner = pair
                .into_inner()
                .next()
                .unwrap()
                .into_inner()
                .next()
                .unwrap();
            res.push_str("Vec<");
            convert_typeid(inner, res);
            res.push_str(">");
        }
        Rule::ident => {
            let ident = pair.as_str();
            let ident = convert_type(ident);
            res.push_str(&ident);
        }
        _ => unreachable!(),
    };
}

fn render_param(
    pair: pest::iterators::Pair<Rule>,
    docinfo: &HashMap<String, ParamDocInfo>,
    parent_class: &str,
) -> quote::Tokens {
    let mut pairs = pair.into_inner();
    let mut name = pairs.next().unwrap().as_str().to_owned();
    let docinfo = docinfo.get(&name).unwrap();
    let typeid = pairs.next().unwrap();
    let typeid = typeid.into_inner().next().unwrap();
    let mut typeid_str = String::new();
    convert_typeid(typeid, &mut typeid_str);
    if typeid_str == parent_class {
        typeid_str = format!("Box<{}>", typeid_str);
    }
    let typeid = typeid_str;
    let mut pre = if name == "type" {
        name.push('_');
        quote! {
            #[serde(rename="type")]
        }
    } else {
        quote::Tokens::new()
    };
    let default_false = if typeid == "bool" {
        quote!{
            #[serde(default)]
        }
    } else {
        quote::Tokens::new()
    };
    let serialize_number = if typeid == "i32" || typeid == "i64" {
        quote!{
            #[serde(deserialize_with="::serde_aux::field_attributes::deserialize_number_from_string")]
        }
    } else {
        quote::Tokens::new()
    };
    let typeid = if docinfo.optional {
        quote::Ident::new(format!("Option<{}>", typeid))
    } else {
        quote::Ident::new(typeid)
    };
    let name = quote::Ident::new(name);
    let doc = docinfo.doc.replace("//-", " ");
    pre.append(quote! {
        #[doc = #doc]
        #serialize_number
        #default_false
        pub #name:#typeid
    });
    pre
}

#[derive(Debug)]
struct Class {
    name: String,
    types: Vec<String>,
    doc: String,
}

fn render_type(
    pair: pest::iterators::Pair<Rule>,
    docinfo: TypeDocInfo,
    classes: &mut HashMap<String, Class>,
) -> quote::Tokens {
    let mut pairs = pair.into_inner();
    let name = pairs.next().unwrap().as_str();
    let name_capitalized = quote::Ident::new(capitalize(name));
    let params = pairs.next().unwrap();
    let classname = capitalize(pairs.next().unwrap().as_str());
    let params = params
        .into_inner()
        .map(|p| render_param(p, &docinfo.params, &classname))
        .collect::<Vec<_>>();
    let class = classes.entry(classname.clone()).or_insert_with(|| Class {
        name: classname,
        types: Vec::new(),
        doc: String::new(),
    });
    class.types.push(capitalize(name));

    let doc = docinfo.doc.replace("//-", " ");
    quote! {
        #[derive(Serialize, Deserialize, Debug, Clone)]
        #[doc = #doc]
        pub struct #name_capitalized {
            #(#params),*
        }
    }
}

fn render_method(pair: pest::iterators::Pair<Rule>, docinfo: TypeDocInfo) -> quote::Tokens {
    let mut pairs = pair.into_inner();
    let name = pairs.next().unwrap().as_str();
    let name_capitalized = capitalize(name);
    let params = pairs.next().unwrap();
    let params = params
        .into_inner()
        .map(|p| render_param(p, &docinfo.params, ""))
        .collect::<Vec<_>>();
    let name_ident = quote::Ident::new(name_capitalized);
    let rettype = quote::Ident::new(convert_type(pairs.next().unwrap().as_str()));

    let doc = docinfo.doc.replace("//-", " ");
    quote! {
        #[derive(Serialize, Deserialize, Debug, Clone)]
        #[doc = #doc]
        pub struct #name_ident {
            #(#params),*
        }
        impl Method for #name_ident {
            const TYPE: &'static str = #name;
            type Response = #rettype;
        }
    }
}

fn render_class(class: Class) -> quote::Tokens {
    let name = quote::Ident::new(class.name);
    let types = class
        .types
        .into_iter()
        .map(|t| quote::Ident::new(t))
        .collect::<Vec<_>>();
    if types.len() <= 1 {
        return quote::Tokens::new();
    }
    let types2 = types.clone();
    let doc = class.doc.replace("//-", " ");
    quote! {
        #[derive(Serialize, Deserialize, Debug, Clone)]
        #[serde(rename_all="camelCase")]
        #[serde(tag="@type")]
        #[doc = #doc]
        pub enum #name {
            #(#types(#types2)),*
        }
    }
}

#[derive(Debug)]
struct ParamDocInfo {
    optional: bool,
    doc: String,
}
#[derive(Debug)]
struct TypeDocInfo {
    doc: String,
    params: HashMap<String, ParamDocInfo>,
}

fn extract_docinfo(
    pair: pest::iterators::Pair<Rule>,
    classes: &mut HashMap<String, Class>,
) -> TypeDocInfo {
    let mut params = HashMap::new();
    let mut doc = String::new();
    for p in pair.into_inner() {
        let mut pairs = p.into_inner();
        let name = pairs.next().unwrap().as_str();
        let descr = pairs.next().unwrap().as_str();
        if name == "description" {
            doc = descr.to_owned();
        } else if name == "class" {
            classes.insert(
                name.to_owned(),
                Class {
                    name: name.to_owned(),
                    types: Vec::new(),
                    doc: descr.to_owned(),
                },
            );
        } else {
            let optional = descr.contains("may be null")
                || descr.contains("only available to bots")
                || descr.contains("bots only")
                || descr.contains("or null");
            let n = if name == "param_description" {
                "description"
            } else {
                name
            };
            params.insert(
                n.to_owned(),
                ParamDocInfo {
                    optional,
                    doc: descr.to_owned(),
                },
            );
        }
    }
    TypeDocInfo { doc, params }
}

pub fn generate(src: &str) -> (String, String) {
    let pairs = TlParser::parse(Rule::tl, &src).unwrap_or_else(|e| panic!("{}", e));

    let mut functions = false;
    let mut classes = HashMap::new();
    let mut type_tokens = quote::Tokens::new();
    let mut method_tokens = quote::Tokens::new();
    for pair in pairs {
        match pair.as_rule() {
            Rule::section => {
                functions = true;
            }
            Rule::definition => {
                let mut pairs = pair.into_inner();
                let docstring = pairs.next().unwrap();
                let typedef = pairs.next().unwrap();
                let docinfo = extract_docinfo(docstring, &mut classes);
                if functions {
                    method_tokens.append(render_method(typedef, docinfo));
                } else {
                    type_tokens.append(render_type(typedef, docinfo, &mut classes));
                }
            }
            _ => {
                unreachable!();
            }
        }
    }
    for (_, class) in classes.into_iter() {
        type_tokens.append(render_class(class));
    }
    (type_tokens.as_str().to_owned(), method_tokens.as_str().to_owned())
}
