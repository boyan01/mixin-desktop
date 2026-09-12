//! Generates the mechanical UniFFI adapter without linking the app or its stale bindings.

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

use quote::{format_ident, quote, ToTokens};
use syn::{
    parse_quote, Fields, FnArg, GenericArgument, ImplItem, ImplItemFn, Item, ItemEnum, ItemFn,
    ItemStruct, Pat, PathArguments, ReturnType, Type, UseTree, Visibility,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const MODULES: [&str; 5] = ["account", "desktop", "login", "media", "model"];

#[derive(Default)]
struct Api {
    records: BTreeMap<String, ItemStruct>,
    enums: BTreeMap<String, ItemEnum>,
    methods: BTreeMap<(String, String), ImplItemFn>,
    public_types: BTreeSet<String>,
}

impl Api {
    fn load(root: &Path) -> Result<Self> {
        let mut api = Self::default();
        // Resolve the facade's DTO re-exports from their current defining modules.
        // These are generator inputs, not dependencies of the Swift bridge.
        for file in [
            "mixin_desktop_api/src/model.rs",
            "mixin_desktop_api/src/access.rs",
            "mixin_desktop_api/src/account.rs",
            "mixin_desktop_api/src/desktop.rs",
            "mixin_desktop_api/src/login.rs",
            "mixin_desktop_api/src/media.rs",
            "mixin_desktop_core/src/runtime/model.rs",
            "mixin_desktop_media/src/player.rs",
            "mixin_desktop_media/src/recorder.rs",
        ] {
            api.read(&fs::read_to_string(root.join(file))?)?;
        }
        let facade = syn::parse_file(&fs::read_to_string(
            root.join("mixin_desktop_api/src/lib.rs"),
        )?)?;
        for item in facade.items {
            if let Item::Use(item) = item {
                if matches!(item.vis, Visibility::Public(_)) {
                    collect_exports(&item.tree, &mut api.public_types)?;
                }
            }
        }
        Ok(api)
    }

    fn read(&mut self, source: &str) -> Result<()> {
        for item in syn::parse_file(source)?.items {
            match item {
                Item::Struct(item) if matches!(item.vis, Visibility::Public(_)) => {
                    let name = item.ident.to_string();
                    if self.records.insert(name.clone(), item).is_some() {
                        return Err(format!("duplicate record: {name}").into());
                    }
                }
                Item::Enum(item) if matches!(item.vis, Visibility::Public(_)) => {
                    let name = item.ident.to_string();
                    if self.enums.insert(name.clone(), item).is_some() {
                        return Err(format!("duplicate enum: {name}").into());
                    }
                }
                Item::Impl(item) if item.trait_.is_none() => {
                    let Type::Path(owner) = *item.self_ty else {
                        continue;
                    };
                    let owner = owner.path.segments.last().unwrap().ident.to_string();
                    for method in item.items {
                        if let ImplItem::Fn(method) = method {
                            if matches!(method.vis, Visibility::Public(_)) {
                                let key = (owner.clone(), method.sig.ident.to_string());
                                if self.methods.insert(key.clone(), method).is_some() {
                                    return Err(format!("duplicate method: {key:?}").into());
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn lower_type(&self, ty: &Type) -> Result<Type> {
        if let Type::Tuple(tuple) = ty {
            if tuple.elems.is_empty() {
                return Ok(ty.clone());
            }
        }
        let Type::Path(path) = ty else {
            return Err(format!("unsupported bridge type: {}", ty.to_token_stream()).into());
        };
        if path.qself.is_some() {
            return Err("qualified self types are unsupported".into());
        }
        let segment = path.path.segments.last().ok_or("empty type path")?;
        let name = segment.ident.to_string();
        let ident = &segment.ident;
        match name.as_str() {
            "Vec" | "Option" | "HashMap" => {
                let PathArguments::AngleBracketed(args) = &segment.arguments else {
                    return Err(format!("missing type arguments: {name}").into());
                };
                let types = args
                    .args
                    .iter()
                    .map(|arg| match arg {
                        GenericArgument::Type(ty) => self.lower_type(ty),
                        _ => Err("only type arguments are supported".into()),
                    })
                    .collect::<Result<Vec<_>>>()?;
                let expected = if name == "HashMap" { 2 } else { 1 };
                if types.len() != expected {
                    return Err(format!("invalid type arguments: {name}").into());
                }
                if name == "HashMap" {
                    Ok(parse_quote!(std::collections::HashMap<#(#types),*>))
                } else {
                    Ok(parse_quote!(#ident<#(#types),*>))
                }
            }
            _ => {
                if !matches!(segment.arguments, PathArguments::None) {
                    return Err(format!("unsupported generic type: {name}").into());
                }
                if matches!(
                    name.as_str(),
                    "String"
                        | "bool"
                        | "i8"
                        | "i16"
                        | "i32"
                        | "i64"
                        | "u8"
                        | "u16"
                        | "u32"
                        | "u64"
                        | "f32"
                        | "f64"
                ) {
                    Ok(parse_quote!(#ident))
                } else if self.public_types.contains(&name) {
                    Ok(parse_quote!(mixin_desktop_api::#ident))
                } else {
                    Err(format!("type is not an exported facade DTO: {name}").into())
                }
            }
        }
    }

    fn remote_type(&self, kind: &str, name: &str) -> Result<String> {
        let (export_name, name) = name.split_once('=').unwrap_or((name, name));
        if !self.public_types.contains(name) {
            return Err(format!("type is not exported by mixin_desktop_api: {name}").into());
        }
        let ident = format_ident!("{name}");
        let declaration = match kind {
            "record" => {
                let mut record = self
                    .records
                    .get(name)
                    .ok_or_else(|| format!("record not found: {name}"))?
                    .clone();
                if !record.generics.params.is_empty() || !matches!(record.fields, Fields::Named(_))
                {
                    return Err(format!("unsupported record shape: {name}").into());
                }
                record.attrs.retain(|attr| attr.path().is_ident("doc"));
                if export_name != name {
                    record
                        .attrs
                        .push(parse_quote!(#[uniffi(name = #export_name)]));
                }
                for field in &mut record.fields {
                    if !matches!(field.vis, Visibility::Public(_))
                        || field.attrs.iter().any(|attr| !attr.path().is_ident("doc"))
                    {
                        return Err(
                            format!("unsupported field attributes or visibility: {name}").into(),
                        );
                    }
                    field.ty = self.lower_type(&field.ty)?;
                }
                quote!(#[uniffi::remote(Record)] #record)
            }
            "enum" => {
                let mut item = self
                    .enums
                    .get(name)
                    .ok_or_else(|| format!("enum not found: {name}"))?
                    .clone();
                if !item.generics.params.is_empty() {
                    return Err(format!("generic enum is unsupported: {name}").into());
                }
                item.attrs.retain(|attr| attr.path().is_ident("doc"));
                if export_name != name {
                    item.attrs
                        .push(parse_quote!(#[uniffi(name = #export_name)]));
                }
                for variant in &item.variants {
                    if variant
                        .attrs
                        .iter()
                        .any(|attr| !attr.path().is_ident("doc"))
                        || variant.discriminant.is_some()
                        || !matches!(variant.fields, Fields::Unit)
                    {
                        return Err(format!("only plain unit enums are supported: {name}").into());
                    }
                }
                quote!(#[uniffi::remote(Enum)] #item)
            }
            _ => return Err(format!("unknown type kind: {kind}").into()),
        };
        Ok(quote!(use mixin_desktop_api::#ident; #declaration).to_string())
    }

    fn forward(&self, owner: &str, receiver: &str, name: &str) -> Result<ItemFn> {
        let (export_name, source_name) = name.split_once('=').unwrap_or((name, name));
        let source = self
            .methods
            .get(&(owner.into(), source_name.into()))
            .ok_or_else(|| format!("method not found: {owner}::{source_name}"))?;
        if source
            .attrs
            .iter()
            .any(|attr| !attr.path().is_ident("doc") && !attr.path().is_ident("allow"))
        {
            return Err(format!("unsupported method attributes: {owner}::{source_name}").into());
        }
        let mut signature = source.sig.clone();
        if !signature.generics.params.is_empty()
            || signature.generics.where_clause.is_some()
            || signature.unsafety.is_some()
            || signature.abi.is_some()
            || signature.variadic.is_some()
            || signature.constness.is_some()
        {
            return Err(format!("unsupported signature: {owner}::{source_name}").into());
        }
        let Some(FnArg::Receiver(this)) = signature.inputs.first() else {
            return Err("only shared instance methods are supported".into());
        };
        if this.reference.is_none() || this.mutability.is_some() || this.colon_token.is_some() {
            return Err("only &self receivers are supported".into());
        }
        let mut args = Vec::new();
        for input in signature.inputs.iter_mut().skip(1) {
            let FnArg::Typed(arg) = input else {
                return Err("unexpected receiver".into());
            };
            let Pat::Ident(pattern) = &*arg.pat else {
                return Err("argument must be an identifier".into());
            };
            if pattern.by_ref.is_some() || pattern.subpat.is_some() || !arg.attrs.is_empty() {
                return Err("unsupported argument pattern or attributes".into());
            }
            args.push(pattern.ident.clone());
            *arg.ty = self.lower_type(&arg.ty)?;
        }
        let mut fallible = false;
        if let ReturnType::Type(_, ty) = &mut signature.output {
            let result = result_value(ty)?;
            *ty = if let Some(value) = result {
                fallible = true;
                let value = self.lower_type(&value)?;
                Box::new(parse_quote!(Result<#value, crate::error::SwiftClientError>))
            } else {
                Box::new(self.lower_type(ty)?)
            };
        }
        let method = format_ident!("{source_name}");
        let receiver: syn::Expr = syn::parse_str(receiver)?;
        let mut call = quote!(#receiver.#method(#(#args),*));
        if signature.asyncness.is_some() {
            call = quote!(#call.await);
        }
        if fallible {
            call = quote!(Ok(#call?));
        }
        signature.ident = format_ident!("{export_name}");
        let mut method: ItemFn = parse_quote!(pub #signature { #call });
        method.attrs = source.attrs.clone();
        Ok(method)
    }
}

fn result_value(ty: &Type) -> Result<Option<Type>> {
    let Type::Path(path) = ty else {
        return Ok(None);
    };
    let segment = path.path.segments.last().ok_or("empty return type")?;
    if segment.ident != "ClientResult" && segment.ident != "Result" {
        return Ok(None);
    }
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return Err("missing result arguments".into());
    };
    let expected = if segment.ident == "ClientResult" {
        1
    } else {
        2
    };
    if arguments.args.len() != expected {
        return Err("unexpected result arguments".into());
    }
    if expected == 2
        && arguments.args.last().unwrap().to_token_stream().to_string() != "ClientError"
    {
        return Err("only ClientError results are supported".into());
    }
    let Some(GenericArgument::Type(value)) = arguments.args.first() else {
        return Err("missing result value".into());
    };
    Ok(Some(value.clone()))
}

fn collect_exports(tree: &UseTree, names: &mut BTreeSet<String>) -> Result<()> {
    match tree {
        UseTree::Path(path) => collect_exports(&path.tree, names)?,
        UseTree::Group(group) => {
            for item in &group.items {
                collect_exports(item, names)?;
            }
        }
        UseTree::Name(name) => {
            names.insert(name.ident.to_string());
        }
        _ => return Err("facade glob/renamed exports require explicit generator support".into()),
    }
    Ok(())
}

fn generate(api: &Api, exports: &str) -> Result<BTreeMap<String, String>> {
    // Emit empty modules too: removing the last export must remove its old adapter.
    let mut output = MODULES
        .into_iter()
        .map(|module| (module.to_string(), String::new()))
        .collect::<BTreeMap<_, _>>();
    let mut seen = BTreeSet::new();
    let mut forwards = BTreeMap::<(String, String), Vec<ItemFn>>::new();
    for (index, line) in exports.lines().enumerate() {
        let line = line.split('#').next().unwrap().trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<_> = line.split_whitespace().collect();
        let (module, key, declaration) = match parts.as_slice() {
            [module, kind @ ("record" | "enum"), name] => (
                *module,
                format!(
                    "type::{}",
                    name.split_once('=').map_or(*name, |(_, source)| source)
                ),
                api.remote_type(kind, name)?,
            ),
            [module, handle, owner, receiver, names] => {
                let handle = syn::parse_str::<syn::Ident>(handle)?;
                let mut methods = Vec::new();
                for name in names.split(',') {
                    let method = api.forward(owner, receiver, name)?;
                    let key = format!("{handle}::{}", method.sig.ident);
                    if !seen.insert(key.clone()) {
                        return Err(format!("duplicate export: {key}").into());
                    }
                    methods.push(method);
                }
                forwards
                    .entry((module.to_string(), handle.to_string()))
                    .or_default()
                    .extend(methods);
                (*module, format!("route::{index}"), String::new())
            }
            _ => return Err(format!("invalid exports line {}: {line}", index + 1).into()),
        };
        if !MODULES.contains(&module) {
            return Err(format!("unknown output module: {module}").into());
        }
        if !seen.insert(key.clone()) {
            return Err(format!("duplicate export: {key}").into());
        }
        output
            .entry(module.into())
            .or_default()
            .push_str(&format!("{declaration}\n"));
    }
    for ((module, handle), methods) in forwards {
        let handle = syn::parse_str::<syn::Ident>(&handle)?;
        let export = if methods.iter().any(|method| method.sig.asyncness.is_some()) {
            quote!(#[uniffi::export(async_runtime = "tokio")])
        } else {
            quote!(#[uniffi::export])
        };
        output
            .entry(module)
            .or_default()
            .push_str(&quote!(#export impl #handle { #(#methods)* }).to_string());
    }
    Ok(output)
}

fn format_source(source: &str) -> Result<String> {
    let mut child = Command::new("rustfmt")
        .args(["--edition", "2021", "--emit", "stdout"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("missing rustfmt stdin")?
        .write_all(source.as_bytes())?;
    let result = child.wait_with_output()?;
    if !result.status.success() {
        return Err("rustfmt failed".into());
    }
    Ok(format!(
        "// Generated by mixin_swift_bridgegen. Do not edit.\n\n{}",
        String::from_utf8(result.stdout)?
    ))
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if !args.is_empty() && args != ["--check"] {
        return Err("usage: cargo run -p mixin_swift_bridgegen -- [--check]".into());
    }
    let check = !args.is_empty();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let api = Api::load(&root)?;
    let exports = fs::read_to_string(root.join("swiftui/bridgegen/exports.txt"))?;
    // Validate and format every output before writing any file.
    let output = generate(&api, &exports)?
        .into_iter()
        .map(|(module, source)| Ok((module, format_source(&source)?)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let directory = root.join("swiftui/rust/src/generated");
    let mut stale = Vec::new();
    for (module, source) in &output {
        let path = directory.join(format!("{module}.rs"));
        if fs::read_to_string(&path).ok().as_ref() == Some(source) {
            continue;
        }
        if check {
            stale.push(module.clone());
        } else {
            fs::create_dir_all(&directory)?;
            fs::write(path, source)?;
        }
    }
    if !stale.is_empty() {
        return Err(format!(
            "stale Swift adapters: {}; run cargo run -p mixin_swift_bridgegen",
            stale.join(", ")
        )
        .into());
    }
    println!(
        "{} {} Swift adapter modules",
        if check { "Checked" } else { "Generated" },
        output.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changed_api_arguments_generate_forwarders_that_preserve_values_and_typed_errors() {
        let source = r#"
            pub struct Client;
            impl Client {
                pub async fn query(&self, term: String, limit: Option<u64>) -> ClientResult<String> {
                    if term.is_empty() { return Err(ClientError); }
                    Ok(format!("{term}:{}", limit.unwrap_or(15)))
                }
                pub fn count(&self) -> u64 { 7 }
            }
        "#;
        let mut api = Api::default();
        api.read(source).unwrap();
        let query = api
            .forward("Client", "self.client", "search=query")
            .unwrap();
        let count = api.forward("Client", "self.client", "count").unwrap();
        // Compile and execute the generated bodies, so signature drift, argument ordering,
        // missing awaits and lost error conversion fail as observable client regressions.
        let fixture = format!(
            r#"
            #[derive(Debug)] struct ClientError;
            type ClientResult<T> = Result<T, ClientError>;
            mod error {{
                #[derive(Debug, PartialEq)] pub enum SwiftClientError {{ InvalidArgument }}
                impl From<super::ClientError> for SwiftClientError {{
                    fn from(_: super::ClientError) -> Self {{ Self::InvalidArgument }}
                }}
            }}
            {source}
            struct Handle {{ client: Client }}
            impl Handle {{ {query} {count} }}
            fn ready<T>(future: impl std::future::Future<Output = T>) -> T {{
                let mut future = std::pin::pin!(future);
                let mut context = std::task::Context::from_waker(std::task::Waker::noop());
                match future.as_mut().poll(&mut context) {{
                    std::task::Poll::Ready(value) => value,
                    _ => panic!("fixture unexpectedly suspended"),
                }}
            }}
            fn main() {{
                let handle = Handle {{ client: Client }};
                assert_eq!(ready(handle.search("hello".into(), Some(42))).unwrap(), "hello:42");
                assert_eq!(ready(handle.search("hello".into(), None)).unwrap(), "hello:15");
                assert_eq!(ready(handle.search(String::new(), None)), Err(error::SwiftClientError::InvalidArgument));
                assert_eq!(handle.count(), 7);
            }}
        "#,
            query = query.to_token_stream(),
            count = count.to_token_stream()
        );
        let dir = std::env::temp_dir().join(format!("mixin-bridgegen-test-{}", std::process::id()));
        fs::create_dir(&dir).unwrap();
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(dir.clone());
        let source_path = dir.join("main.rs");
        let executable = dir.join("fixture");
        fs::write(&source_path, fixture).unwrap();
        let compile = Command::new("rustc")
            .arg("--edition=2021")
            .arg(source_path)
            .arg("-o")
            .arg(&executable)
            .output()
            .unwrap();
        assert!(
            compile.status.success(),
            "{}",
            String::from_utf8_lossy(&compile.stderr)
        );
        let run = Command::new(executable).output().unwrap();
        assert!(
            run.status.success(),
            "{}",
            String::from_utf8_lossy(&run.stderr)
        );
    }

    #[test]
    fn added_dto_field_is_exported_with_its_type_and_original_order() {
        let mut api = Api::default();
        api.public_types.insert("Profile".into());
        api.read("pub struct Profile { pub id: String, pub labels: Option<Vec<String>> }")
            .unwrap();
        let generated = syn::parse_file(&api.remote_type("record", "Profile").unwrap()).unwrap();
        let Item::Struct(record) = &generated.items[1] else {
            panic!("missing record")
        };
        let fields = record.fields.iter().collect::<Vec<_>>();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].ident.as_ref().unwrap(), "id");
        assert_eq!(fields[1].ident.as_ref().unwrap(), "labels");
        assert_eq!(fields[1].ty, parse_quote!(Option<Vec<String>>));
    }

    #[test]
    fn documented_fields_and_variants_keep_their_docs_and_types() {
        let mut api = Api::default();
        api.public_types.extend(["Profile".into(), "Status".into()]);
        api.read(
            "/// A profile.\npub struct Profile {\n/// Stable ID.\npub id: String }\n\
                  pub enum Status {\n/// Ready to use.\nReady }",
        )
        .unwrap();
        let record = syn::parse_file(&api.remote_type("record", "Profile").unwrap()).unwrap();
        let Item::Struct(record) = &record.items[1] else {
            panic!("missing record")
        };
        assert!(record.attrs.iter().any(|attr| attr.path().is_ident("doc")));
        let field = record.fields.iter().next().unwrap();
        assert_eq!(field.ty, parse_quote!(String));
        assert_eq!(field.attrs, vec![parse_quote!(#[doc = " Stable ID."])]);
        let item = syn::parse_file(&api.remote_type("enum", "Status").unwrap()).unwrap();
        let Item::Enum(item) = &item.items[1] else {
            panic!("missing enum")
        };
        assert_eq!(
            item.variants[0].attrs,
            vec![parse_quote!(#[doc = " Ready to use."])]
        );
    }

    #[test]
    fn conditional_fields_and_variants_require_explicit_adapter_support() {
        let mut api = Api::default();
        api.public_types.extend(["Profile".into(), "Status".into()]);
        api.read(
            "pub struct Profile { #[cfg(feature = \"extra\")] pub id: String }\n\
                  pub enum Status { #[cfg(feature = \"extra\")] Ready }",
        )
        .unwrap();
        assert!(api.remote_type("record", "Profile").is_err());
        assert!(api.remote_type("enum", "Status").is_err());
    }

    #[test]
    fn unsupported_api_shapes_fail_instead_of_silently_changing_the_contract() {
        for source in [
            "pub async fn query(&self) -> impl Stream<Item = String> { todo!() }",
            "pub fn query<T>(&self, value: T) {}",
            "pub fn query(&mut self) {}",
            "pub fn query(&self, value: &str) {}",
            "pub fn query(&self) -> Result<String, OtherError> { todo!() }",
            "#[cfg(feature = \"optional\")] pub fn query(&self) {}",
        ] {
            let mut api = Api::default();
            api.read(&format!("impl Client {{ {source} }}")).unwrap();
            assert!(
                api.forward("Client", "self.client", "query").is_err(),
                "{source}"
            );
        }
    }

    #[test]
    fn unselected_api_methods_stay_private_and_missing_or_duplicate_exports_fail() {
        let mut api = Api::default();
        api.read("impl Client { pub fn selected(&self) {} pub fn other(&self) {} }")
            .unwrap();
        let route = "login Handle Client self.client selected";
        let generated = generate(&api, route).unwrap();
        let file = syn::parse_file(&generated["login"]).unwrap();
        let Item::Impl(item) = &file.items[0] else {
            panic!("missing impl")
        };
        assert_eq!(item.items.len(), 1);
        let ImplItem::Fn(method) = &item.items[0] else {
            panic!("missing method")
        };
        assert_eq!(method.sig.ident, "selected");
        assert!(generate(&api, &format!("{route}\n{route}")).is_err());
        assert!(generate(&api, "login Handle Client self.client missing").is_err());
        let removed = generate(&api, "").unwrap();
        assert_eq!(removed.len(), MODULES.len());
        assert!(removed["login"].is_empty());
    }
}
