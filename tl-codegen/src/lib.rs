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

fn convert_type(t: &str) -> proc_macro2::Ident {
    format_ident!("{}", match t {
        "double" => "f64".to_owned(),
        "string" => "String".to_owned(),
        "int32" => "i32".to_owned(),
        "int53" => "i64".to_owned(),
        "int64" => "i64".to_owned(),
        "Bool" => "bool".to_owned(),
        "bytes" => "String".to_owned(),
        _ => capitalize(t),
    })
}

fn convert_typeid(pair: pest::iterators::Pair<Rule>) -> proc_macro2::TokenStream {
    match pair.as_rule() {
        Rule::vector => {
            let inner = pair
                .into_inner()
                .next()
                .unwrap()
                .into_inner()
                .next()
                .unwrap();
            let t = convert_typeid(inner);
            quote! {Vec<#t>}
        }
        Rule::ident => {
            let ident = pair.as_str();
            let t = convert_type(ident);
            quote! { #t }
        }
        _ => unreachable!(),
    }
}

fn render_param(
    pair: pest::iterators::Pair<Rule>,
    docinfo: &HashMap<String, ParamDocInfo>,
    parent_class: &str,
) -> proc_macro2::TokenStream {
    let mut pairs = pair.into_inner();
    let mut name = pairs.next().unwrap().as_str().to_owned();
    let docinfo = docinfo.get(&name).unwrap();
    let typeid = pairs.next().unwrap();
    let typeid = typeid.into_inner().next().unwrap();
    let mut typeid = convert_typeid(typeid);
    let typeid_str = format!("{}", typeid);
    if typeid_str == parent_class {
        typeid = quote!{ Box<#typeid> };
    }
    let mut pre = if name == "type" {
        name.push('_');
        quote! {
            #[serde(rename="type")]
        }
    } else {
        quote! {}
    };
    let default_false = if typeid_str == "bool" {
        quote!{
            #[serde(default)]
        }
    } else {
        quote! {}
    };
    let serialize_number = if typeid_str == "i32" || typeid_str == "i64" {
        quote!{
            #[serde(deserialize_with="::serde_aux::field_attributes::deserialize_number_from_string")]
        }
    } else {
        quote! {}
    };
    let typeid = if docinfo.optional {
        quote!{Option<#typeid>}
    } else {
        quote!{#typeid}
    };
    let name = format_ident!("{}", name);
    let doc = docinfo.doc.replace("//-", " ");
    pre.extend(quote! {
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
) -> proc_macro2::TokenStream {
    let mut pairs = pair.into_inner();
    let name = pairs.next().unwrap().as_str();
    let name_capitalized = format_ident!("{}", capitalize(name));
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

fn render_method(pair: pest::iterators::Pair<Rule>, docinfo: TypeDocInfo) -> proc_macro2::TokenStream {
    let mut pairs = pair.into_inner();
    let name = pairs.next().unwrap().as_str();
    let name_capitalized = capitalize(name);
    let params = pairs.next().unwrap();
    let params = params
        .into_inner()
        .map(|p| render_param(p, &docinfo.params, ""))
        .collect::<Vec<_>>();
    let name_ident = format_ident!("{}",name_capitalized);
    let rettype = convert_type(pairs.next().unwrap().as_str());

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

fn render_class(class: Class) -> proc_macro2::TokenStream {
    let name = format_ident!("{}",class.name);
    let types = class
        .types
        .into_iter()
        .map(|t| format_ident!("{}",t))
        .collect::<Vec<_>>();
    if types.len() <= 1 {
        return quote!{};
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
    let mut type_tokens = quote!{};
    let mut method_tokens = quote!{};
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
                    method_tokens.extend(render_method(typedef, docinfo));
                } else {
                    type_tokens.extend(render_type(typedef, docinfo, &mut classes));
                }
            }
            _ => {
                unreachable!();
            }
        }
    }
    for (_, class) in classes.into_iter() {
        type_tokens.extend(render_class(class));
    }
    (format!("{}",type_tokens), format!("{}",method_tokens))
}
