#![deny(private_in_public, unused_extern_crates)]
#![recursion_limit = "128"]

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

use proc_macro2::TokenStream as Tokens;

use parser::{Constructor, Delimiter, Field, Item, Matched, NameChunks, Type};
use quote::quote;


pub mod parser {
    use std::cmp::Ordering;

    use pom::{self, Parser};
    use pom::char_class::{alphanum, digit, hex_digit};
    use pom::parser::*;

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    pub enum Type {
        Int,
        Flags,
        Named(Vec<String>),
        TypeParameter(String),
        Generic(Vec<String>, Box<Type>),
        Flagged(String, u32, Box<Type>),
        Repeated(Option<u32>, Vec<Field>),
    }

    impl Type {
        fn get_dotted_name(dotted_ident: &[String]) -> String {
            dotted_ident.join(".")
        }

        fn get_generic_name(dotted_ident: &[String], ty: &Type) -> String {
           Self::get_dotted_name(dotted_ident) + " " + ty.get_name().as_str()
        }

        fn get_flagged_name(name: &str, bit: u32, ty: &Type) -> String {
            name.to_owned() + "." + bit.to_string().as_str() + "?" + ty.get_name().as_str()
        }

        fn get_repeated_name(repeat_count: Option<u32>, fields: &[Field]) -> String {
            let mut result = String::new();
            if let Some(value) = repeat_count {
                result = format!("{} * ", value);
            }
            result += "[ ";
            for field in fields {
                result += &(field.get_str() + " ");
            }
            result + "]"
        }

        pub fn get_name(&self) -> String {
            use self::Type::*;
            match *self {
                Int => "#".to_owned(),
                Flags => "flags".to_owned(),
                Named(ref v) => Self::get_dotted_name(&v),
                TypeParameter(ref ty) => ty.clone(),
                Generic(ref v, ref ty) => Self::get_generic_name(&v, &ty),
                Flagged(ref name, ref bit, ref ty) => Self::get_flagged_name(&name, *bit, &ty),
                Repeated(repeat_count, ref fields) => Self::get_repeated_name(repeat_count, &fields),
            }
        }

        pub fn names_vec(&self) -> Option<&Vec<String>> {
            use self::Type::*;
            match *self {
                Int |
                Flags |
                TypeParameter(..) |
                Flagged(..) |
                Repeated(..) => None,
                Named(ref v) |
                Generic(ref v, ..) => Some(v),
            }
        }

        pub fn names_vec_mut(&mut self) -> Option<&mut Vec<String>> {
            use self::Type::*;
            match *self {
                Int |
                Flags |
                TypeParameter(..) |
                Flagged(..) |
                Repeated(..) => None,
                Named(ref mut v) |
                Generic(ref mut v, ..) => Some(v),
            }
        }

        pub fn owned_names_vec(&self) -> Vec<String> {
            self.names_vec().cloned().unwrap_or_else(Vec::new)
        }

        pub fn namespaces(&self) -> &[String] {
            self.names_vec()
                .map(|v| &v[..(v.len() - 1).max(0)])
                .unwrap_or(&[])
        }

        pub fn name(&self) -> Option<&str> {
            self.names_vec().and_then(|v| v.last().map(String::as_str))
        }

        pub fn flag_field(&self) -> Option<(String, u32)> {
            use self::Type::*;
            match self {
                &Flagged(ref f, b, _) => Some((f.clone(), b)),
                _ => None,
            }
        }

        pub fn is_type_parameter(&self) -> bool {
            use self::Type::*;
            match self {
                &TypeParameter(..) => true,
                _ => false,
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    pub struct Field {
        pub name: Option<String>,
        pub ty: Type,
    }

    impl Field {
        pub fn get_str(&self) -> String {
            match self.name {
                Some(ref name) => name.clone() + ":" + self.ty.get_name().as_str(),
                None => self.ty.get_name(),
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    pub struct Constructor<Ty, Fi> {
        pub variant: Ty,
        pub tl_id: Option<u32>,
        pub type_parameters: Vec<Fi>,
        pub fields: Vec<Fi>,
        pub output: Ty,
        pub original_variant: String,
        pub original_output: String,
        pub is_function: bool,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum Delimiter {
        Types,
        Functions,
    }

    #[derive(Debug, Clone)]
    pub enum Item {
        Delimiter(Delimiter),
        Constructor(Constructor<Type, Field>),
        Layer(u32),
    }

    #[derive(Debug, Clone)]
    pub struct Matched<T>(pub T, pub String);

    impl Eq for Matched<Constructor<Type, Field>> {
    }

    impl PartialEq for Matched<Constructor<Type, Field>> {
        fn eq(&self, other: &Self) -> bool {
            PartialEq::eq(&self.1, &other.1)
        }
    }

    impl Ord for Matched<Constructor<Type, Field>> {
        fn cmp(&self, other: &Self) -> Ordering {
            Ord::cmp(&self.1, &other.1)
        }
    }

    impl PartialOrd for Matched<Constructor<Type, Field>> {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            PartialOrd::partial_cmp(&self.1, &other.1)
        }
    }

    fn utf8(v: Vec<u8>) -> String {
        String::from_utf8(v).unwrap()
    }

    fn simple_space() -> Parser<u8, ()> {
	    one_of(b" \t\r\n").repeat(0..).discard()
    }

    fn ident() -> Parser<u8, String> {
        (is_a(alphanum) | sym(b'_')).repeat(1..).map(utf8)
    }

    fn dotted_ident() -> Parser<u8, Vec<String>> {
        ((ident() - sym(b'.')).repeat(0..) + ident())
            .map(|(mut v, i)| {
                v.push(i);
                v
            })
    }

    fn tl_id() -> Parser<u8, u32> {
        sym(b'#') * is_a(hex_digit).repeat(0..9).convert(|s| u32::from_str_radix(&utf8(s), 16))
    }

    fn decimal() -> Parser<u8, u32> {
        is_a(digit).repeat(0..).convert(|s| utf8(s).parse())
    }

    fn ty_flag() -> Parser<u8, Type> {
        (ident() - sym(b'.') + decimal() - sym(b'?') + call(ty))
            .map(|((name, bit), ty)| Type::Flagged(name, bit, Box::new(ty)))
    }

    fn ty_generic() -> Parser<u8, Type> {
        (sym(b'(') * dotted_ident() - simple_space() + call(ty) - sym(b')')).map(|(name, ty)| Type::Generic(name, Box::new(ty))) |
        (dotted_ident() - sym(b'<') + call(ty) - sym(b'>')).map(|(name, ty)| Type::Generic(name, Box::new(ty)))
    }

    fn ty() -> Parser<u8, Type> {
        sym(b'#').map(|_| Type::Int) |
            sym(b'!') * ident().map(Type::TypeParameter) |
            ty_flag() |
            ty_generic() |
            dotted_ident().map(Type::Named)
    }

    fn ty_space_generic() -> Parser<u8, Type> {
        let space_generic = dotted_ident() - simple_space() + ty();
        space_generic.map(|(name, ty)| Type::Generic(name, Box::new(ty))) |
         ty()
    }

    fn base_field() -> Parser<u8, Field> {
        (ident() - sym(b':') + ty())
            .map(|(name, ty)| Field { name: Some(name), ty })
            .name("field")
    }

    fn repeated_field() -> Parser<u8, Field> {
        ((decimal() - simple_space() * sym(b'*') * simple_space()).opt()
            + sym(b'[')
            * call(base_fields)
            - seq(b" ]"))
            .map(|(repeat_count, fv)|
                Field { name: None, ty: Type::Repeated(repeat_count, fv) }
            )
    }

    fn base_field_anonymous_or_repeated() -> Parser<u8, Field> {
        repeated_field() |
            base_field() |
            ty().map(|ty| Field { name: None, ty })
    }

    fn base_fields() -> Parser<u8, Vec<Field>> {
        (simple_space() * base_field_anonymous_or_repeated()).repeat(0..)
    }

    fn ty_param_field() -> Parser<u8, Field> {
        sym(b'{') * base_field() - sym(b'}')
    }

    fn fields() -> Parser<u8, (Vec<Field>, Vec<Field>)> {
        (simple_space().opt() * sym(b'?')).map(|_| (vec![], vec![])) |
            (simple_space() * ty_param_field()).repeat(0..) + base_fields()
    }

    fn output_and_matched<T: 'static>(inner: Parser<u8, T>) -> Parser<u8, Matched<T>> {
        Parser::new(move |input| {
            let start = input.position();
            let output = inner.parse(input)?;
            let end = input.position();
            Ok(Matched(output, utf8(input.segment(start, end))))
        })
    }

   fn get_tl_id(
        parsed_id: Option<u32>,
        variant: String,
        type_parameters: Vec<Field>,
        fields: Vec<Field>,
        output: String) -> Option<u32> 
   {
            match parsed_id {
               Some(id) => Some(id),
               None => {
                    let mut string = variant;
                    for ty in type_parameters {
                        string += &(" {".to_owned() + ty.get_str().as_str() + "}");
                    }
                    for field in fields {
                        string += " ";
                        string += field.get_str().as_str();
                    }

                    string += " = ";
                    string += &output;

                    Some(crc::crc32::checksum_ieee(string.as_bytes()))
               }
            }
    }

    fn constructor() -> Parser<u8, Constructor<Type, Field>> {
        (output_and_matched(dotted_ident()) + tl_id().opt() + fields() - simple_space() - sym(b'=') - simple_space() + output_and_matched(ty_space_generic()) - sym(b';'))
            .map(|(((variant, tl_id), (type_parameters, fields)), output)| Constructor {
                tl_id: get_tl_id(tl_id, variant.1.clone(), type_parameters.clone(), fields.clone(), output.1.clone()),
                type_parameters, fields,
                original_variant: variant.1,
                variant: Type::Named(variant.0),
                original_output: output.1,
                output: output.0,
                is_function: false,
            })
            .name("constructor")
    }

    fn delimiter() -> Parser<u8, Delimiter> {
        seq(b"---types---").map(|_| Delimiter::Types) |
            seq(b"---functions---").map(|_| Delimiter::Functions)
    }

    fn layer() -> Parser<u8, u32> {
        seq(b"// LAYER ") * decimal()
    }

    fn space() -> Parser<u8, ()> {
        let end_comment = || seq(b"*/");
        ( one_of(b" \t\r\n").discard() |
          (seq(b"//") - !(seq(b" LAYER ")) - none_of(b"\r\n").repeat(0..)).discard() |
          (seq(b"/*") * (!end_comment() * take(1)).repeat(0..) * end_comment()).discard()
        ).repeat(0..).discard()
    }

    fn item() -> Parser<u8, Matched<Item>> {
        output_and_matched({
            delimiter().map(Item::Delimiter) |
            constructor().map(Item::Constructor) |
            layer().map(Item::Layer)
        }) - space()
    }

    fn lines() -> Parser<u8, Vec<Matched<Item>>> {
        space() * item().repeat(0..) - end()
    }

    pub fn parse_string(input: &str) -> Result<Vec<Matched<Item>>, pom::Error> {
        let mut input = pom::DataInput::new(input.as_bytes());
        lines().parse(&mut input)
    }

    fn name_ident() -> Parser<u8, NameChunks> {
        ident().map(|s| NameChunks(vec![s]))
    }

    #[derive(Debug, Clone)]
    pub struct NameChunks(pub Vec<String>);

    impl NameChunks {
        pub fn from_name(name: &str) -> Result<Self, pom::Error> {
            let mut input = pom::DataInput::new(name.as_bytes());
            (name_ident() - end()).parse(&mut input)
        }

        pub fn as_snake_case(&self) -> String {
            let names: Vec<String> = self.0.iter()
                .map(|s| s.to_ascii_lowercase())
                .collect();
            names.join("_")
        }

        pub fn as_upper_camel_case(&self) -> String {
            self.0.iter()
                .cloned()
                .map(|mut s| {
                    &mut s[..1].make_ascii_uppercase();
                    s
                })
                .collect()
        }

        pub fn common_prefix_of(&self, other: &Self) -> Self {
            let index = self.0.iter()
                .zip(&other.0)
                .enumerate()
                .skip_while(|&(_, (a, b))| a == b)
                .next()
                .map(|(e, _)| e)
                .unwrap_or(self.0.len().min(other.0.len()));
            NameChunks((&self.0[..index]).to_vec())
        }

        pub fn trim_common_prefix(&mut self, other: &Self) {
            assert!(self.0.starts_with(&other.0), "{:?} not a prefix of {:?}", self, other);
            self.0.drain(0..(other.0.len()));
        }
    }
}

fn fail_hard() -> Tokens {
    quote!(FAIL_LOUDLY_AT_COMPILE_TIME!())
}

#[derive(Debug)]
struct Constructors<Ty, Fi>(Vec<Matched<Constructor<Ty, Fi>>>);

impl<Ty, Fi> Default for Constructors<Ty, Fi> {
    fn default() -> Self {
        Constructors(Vec::new())
    }
}

type TypeResolutionMap = BTreeMap<Vec<String>, TypeIR>;

#[derive(Debug)]
enum NamespaceItem {
    AsEnum(Constructors<TypeIR, FieldIR>),
    AsVariant(Matched<Constructor<TypeIR, FieldIR>>),
    AsFunction(Matched<Constructor<TypeIR, FieldIR>>),
    AnotherNamespace(Namespace),
}

fn write_to_file(contents: String, filename: &Path, append: bool) {
    let mut options = OpenOptions::new();
    options.create(true)
        .write(true)
        .truncate(!append)
        .append(append);

    let mut file = options.open(filename)
        .expect(
            format!(
                "Unable to open file <{filename}> with the given parameters: {options:?}",
                filename = filename.to_string_lossy(),
                options = options
            ).as_str()
        );

    file.write_all(contents.as_bytes())
        .expect(
            format!("Unable to write contents into the file: {}", filename.to_string_lossy()).as_str()
        );
}

fn reformat(filename: &Path) {
    let status = Command::new("rustfmt")
        .arg("--edition")
        .arg("2018")
        .arg(filename)
        .status()
        .expect("Failed to execute rustfmt");
    if !status.success() {
        panic!("Rustfmt return code: {}", status.code().unwrap())
    }
}

#[derive(Debug, Default)]
struct Namespace(BTreeMap<syn::Ident, NamespaceItem>);

impl Namespace {
    fn descend_tree(&mut self, names: &[syn::Ident]) -> &mut Self {
        use self::NamespaceItem::*;
        names.iter()
            .fold(self, |ns, name| {
                match ns.0.entry(name.clone()).or_insert_with(|| AnotherNamespace(Default::default())) {
                    &mut AnotherNamespace(ref mut ns) => ns,
                    other => panic!("descend_tree: duplicate namespace item {} {:?} {:?}", name, other, names),
                }
            })
    }

    fn insert(&mut self, mut names: Vec<syn::Ident>, item: NamespaceItem) {
        let leaf = names.pop().unwrap();
        let namespace = self.descend_tree(&names);
        if namespace.0.contains_key(&leaf) {
            println!("cargo:warning=insert: duplicate namespace item {:?}", names);
            return;
        }
        namespace.0.insert(leaf, item);
    }

    fn print_rust(&self, prelude: Option<String>, path: &Path, append: bool) -> (PathBuf, PathBuf) {
        let has_submodules = AtomicBool::new(false);
        let items = self.0.iter()
            .map(|(name, item)| {
                match item {
                    NamespaceItem::AsEnum(ref cs) => cs.as_enum(),
                    NamespaceItem::AsVariant(ref cm) => cm.0.as_variant_type_struct(&cm.1),
                    NamespaceItem::AsFunction(ref cm) => cm.0.as_function_struct(&cm.1),
                    NamespaceItem::AnotherNamespace(ref ns) => {
                        let prelude = quote! {
                            use serde_derive::{Serialize, Deserialize};
                        }.to_string();
                        let (filename, _dir) = ns.print_rust(Some(prelude), path.join(name.to_string()).as_path(), false);
                        reformat(&filename);
                        has_submodules.store(true, Ordering::Relaxed);

                        quote! { pub mod #name; }
                    },
                }
            });

        let contents = quote!(#( #items )*).to_string();

        let filename = if has_submodules.load(Ordering::Relaxed) {
            path.join("mod.rs")
        } else {
            path.with_extension("rs")
        };

        let dir = filename.parent().expect(
            format!("Unable to get parent directory for: {}", filename.to_string_lossy()).as_str()
        ).to_path_buf();

        std::fs::create_dir_all(&dir).expect(
            format!("Unable to create directory: {}", filename.to_string_lossy()).as_str()
        );

        let write_prelude = if let Some(prelude) = prelude {
            write_to_file(prelude, &filename, append);
            true
        } else {
            false
        };
        write_to_file(contents, &filename, append || write_prelude);
        (filename, dir)
    }

    fn populate_all_constructors<'this>(&'this self, to_populate: &mut Vec<&'this Constructor<TypeIR, FieldIR>>) {
        use self::NamespaceItem::*;
        for item in self.0.values() {
            match *item {
                AsEnum(ref cs) => {
                    to_populate.extend(cs.0.iter().map(|cm| &cm.0));
                },
                AsFunction(ref c) => {
                    to_populate.push(&c.0);
                },
                AnotherNamespace(ref ns) => {
                    ns.populate_all_constructors(to_populate);
                },
                _ => (),
            };
        }
    }
}

#[derive(Debug)]
struct AllConstructors {
    items: Namespace,
    layer: u32,
}

fn filter_items(iv: &mut Vec<Matched<Item>>) {
    let built_in_types: HashSet<&'static str> = [
        "int", "int32", "int53", "int64", "int128", "int256",
        "long", "double", "bytes", "vector",
        "string", "object", "function", "Object", "Function",
        "secureString", "secureBytes", "true",
    ].iter().cloned().collect();

    let test_entities: HashSet<&'static str> = [
        "TestObject", "testObject", "testInt", "testString", "testVectorBytes", "getTestObject"
    ].iter().cloned().collect();

    iv.retain(|&Matched(ref i, _)| {
        let c = match i {
            &Item::Constructor(ref c) => c,
            _ => return true,
        };
        // Blacklist some annoying inconsistencies.
        !(built_in_types.contains(&c.original_variant.as_str()) || test_entities.contains(&c.original_variant.as_str()))
    });
}

impl AllConstructors {
    fn from_matched_items(iv: Vec<Matched<Item>>) -> Self {
        use self::NamespaceItem::*;

        let mut current = Delimiter::Types;
        let mut ret = AllConstructors {
            items: Default::default(),
            layer: 0,
        };
        let mut constructors_tree: BTreeMap<Vec<String>, Constructors<Type, Field>> = BTreeMap::new();
        let mut functions: Vec<Matched<Constructor<Type, Field>>> = Vec::new();
        for Matched(item, text) in iv {
            match item {
                Item::Delimiter(delimiter) => current = delimiter,
                Item::Constructor(mut constructor) => {
                    match current {
                        Delimiter::Types => {
                            let vec = &mut constructors_tree.entry(constructor.output.owned_names_vec())
                                .or_insert_with(Default::default)
                                .0;
                            let cons = Matched(constructor, text);
                            if let Err(index) = vec.binary_search(&cons) {
                                vec.insert(index, cons);
                            }
                        },
                        Delimiter::Functions => {
                            constructor.is_function = true;
                            functions.push(Matched(constructor, text));
                        },
                    }
                },
                Item::Layer(layer_index) => ret.layer = layer_index,
            }
        }
        let mut resolve_map: TypeResolutionMap = Default::default();
        for (_, cs) in &mut constructors_tree {
            let base_ns = cs.first_constructor().output.namespaces().to_vec();
            cs.fix_names(&base_ns, &mut resolve_map);
        }
        for &mut Matched(ref mut c, _) in &mut functions {
            camelize(&mut resolve_map, &mut c.variant, |_, ns| {
                ns.insert(0, "rpc".to_string());
                false
            });
        }
        for (_, cs) in constructors_tree {
            let cs = cs.resolve(&resolve_map);
            for &Matched(ref c, ref m) in &cs.0 {
                ret.items.insert(
                    c.variant.owned_names_vec(),
                    AsVariant(Matched(c.clone(), m.clone())));
            }
            ret.items.insert(cs.first_constructor().output.owned_names_vec(), AsEnum(cs));
        }
        for Matched(c, m) in functions {
            let c = c.resolve(Delimiter::Functions, &resolve_map);
            ret.items.insert(c.variant.owned_names_vec(), AsFunction(Matched(c, m)));
        }
        ret
    }

    fn as_lazy_statics(&self) -> Tokens {
        let mut all_constructors = Default::default();
        self.items.populate_all_constructors(&mut all_constructors);
        let dynamic_deserializers = all_constructors.iter()
            .filter_map(|c| c.as_dynamic_deserializer());

        quote! {
            fn make_deserializers() -> ::std::vec::Vec<crate::DynamicDeserializer> {
                vec![ #( #dynamic_deserializers ),* ]
            }

            lazy_static::lazy_static! {
                static ref ALL_DESERIALIZERS: ::std::vec::Vec<crate::DynamicDeserializer> = make_deserializers();

                pub static ref BY_NUMBER:
                ::std::collections::BTreeMap<crate::ConstructorNumber, &'static crate::DynamicDeserializer> =
                    ALL_DESERIALIZERS.iter()
                        .map(|d| (d.id, d))
                        .collect();

                pub static ref BY_NAME:
                ::std::collections::BTreeMap<&'static str, &'static crate::DynamicDeserializer> =
                    ALL_DESERIALIZERS.iter()
                        .map(|d| (d.type_name, d))
                        .collect();
            }
        }
    }

    fn print_tokens(&self, prelude: Option<String>, path: &Path) {
        let (filename, dir) = self.items.print_rust(prelude, path, false);
        let dynamic_deserializers = self.as_lazy_statics().to_string();
        write_to_file(dynamic_deserializers, &dir.join("dynamic.rs"), false);
        write_to_file(quote! { pub mod dynamic; }.to_string(), &filename, true);
        reformat(&filename)
    }
}

fn no_conflict_ident(s: &str) -> syn::Ident {
    let mut candidate: String = s.into();
    loop {
        match syn::parse_str(&candidate) {
            Ok(i) => return i,
            Err(_) => candidate.push('_'),
        }
    }
}

fn no_conflict_local_ident(s: &str) -> Option<syn::Ident> {
    match s {
        "bytes" => Some(syn::parse_str("bytes_").unwrap()),
        _ => None,
    }
}

fn wrap_option_type(wrap: bool, ty: Tokens) -> Tokens {
    if wrap {
        quote! { Option<#ty> }
    } else {
        ty
    }
}

fn wrap_option_value(wrap: bool, ty: Tokens) -> Tokens {
    if wrap {
        quote! { Some(#ty) }
    } else {
        ty
    }
}

#[derive(Debug, Clone)]
struct TypeName {
    tokens: Tokens,
    tokens_canon: String,
    idents: Option<Vec<syn::Ident>>,
}

impl PartialEq for TypeName {
    fn eq(&self, other: &Self) -> bool {
        self.tokens_canon == other.tokens_canon
    }
}

impl Eq for TypeName {}

impl PartialOrd for TypeName {
    fn partial_cmp(&self, other: &Self) -> Option<::std::cmp::Ordering> {
        self.tokens_canon.partial_cmp(&other.tokens_canon)
    }
}

impl Ord for TypeName {
    fn cmp(&self, other: &Self) -> ::std::cmp::Ordering {
        self.tokens_canon.cmp(&other.tokens_canon)
    }
}

impl TypeName {
    fn transformed_tokens<F>(&self, func: F) -> Tokens
        where F: FnOnce(&Tokens) -> Tokens,
    {
        func(&self.tokens)
    }

    fn transformed<F>(&self, func: F) -> Self
        where F: FnOnce(&Tokens) -> Tokens,
    {
        let tokens = self.transformed_tokens(func);
        let tokens_canon = format!("{}", tokens);
        TypeName { tokens, tokens_canon, idents: None }
    }
}

fn to_snake_case(ident: &str) -> String {
    let mut result = String::new();
    for c in ident.chars() {
        if c.is_ascii_uppercase() && result.len() > 0 {
            result.push_str(format!("_{}", c.to_ascii_lowercase()).as_str());
        } else {
            result.push(c.to_ascii_lowercase());
        }
    }
    result
}

impl<S> ::std::iter::FromIterator<S> for TypeName
    where S: AsRef<str>
{
    fn from_iter<T>(iter: T) -> Self
        where T: IntoIterator<Item = S>,
    {
        let mut tokens = quote!(crate::ton);
        let mut idents = vec![];
        let mut iter = iter.into_iter();
        if let Some(mut last_segment) = iter.next() {
            while let Some(segment) = iter.next() {
                let ident = no_conflict_ident(to_snake_case(last_segment.as_ref()).as_str());
                idents.push(ident);
                last_segment = segment;
            }
            idents.push(no_conflict_ident(last_segment.as_ref()));
        }

        for ident in &idents {
            tokens = quote!(#tokens::#ident);
        }
        let tokens_canon = format!("{}", tokens);

        TypeName { tokens, tokens_canon, idents: Some(idents) }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum WireKind {
    Bare(TypeName),
    Boxed(TypeName),
    TypeParameter(syn::Ident),
    FlaggedTrue,
    Flags,
    ExtraDefault(TypeName),
}

fn is_first_char_lowercase(s: &str) -> bool {
    s.chars().next()
        .map(char::is_lowercase)
        .unwrap_or(false)
}

impl WireKind {
    fn from_names_and_hint(names: &[String], force_bare: bool) -> Self {
        use self::WireKind::*;
        match names.last().map(String::as_str) {
            Some("true") => FlaggedTrue,
            Some(s) if force_bare || is_first_char_lowercase(s) =>
                Bare(names.iter().map(String::as_str).collect()),
            Some(_) =>
                Boxed(names.iter().map(String::as_str).collect()),
            None => unimplemented!(),
        }
    }

    fn type_parameter(id: syn::Ident) -> Self {
        WireKind::TypeParameter(id)
    }

    fn become_container_for(&mut self, include_determiner: bool, contained: Self) {
        use self::WireKind::*;
        let ty_loc = match *self {
            Bare(ref mut t) |
            Boxed(ref mut t) => t,
            _ => unimplemented!(),
        };
        let contained = if include_determiner {
            match contained {
                Bare(ty) => ty.transformed_tokens(|t| quote!(crate::ton::Bare, #t)),
                Boxed(ty) => ty.transformed_tokens(|t| quote!(crate::ton::Boxed, #t)),
                TypeParameter(t) => quote!(crate::ton::Boxed, #t),
                _ => unimplemented!(),
            }
        } else {
            match contained {
                Bare(t) | Boxed(t) => t.tokens,
                TypeParameter(t) => quote!(#t),
                _ => unimplemented!(),
            }
        };
        *ty_loc = ty_loc.transformed(|ty| quote!(#ty<#contained>));
    }

    fn as_read_method(&self) -> Tokens {
        use self::WireKind::*;
        match *self {
            Bare(..) | Flags => quote!(read_bare),
            Boxed(..) | TypeParameter(..) => quote!(read_boxed),
            ExtraDefault(..) => quote!(just_default),
            FlaggedTrue => fail_hard(),
        }
    }

    fn as_write_method(&self) -> Option<Tokens> {
        use self::WireKind::*;
        match *self {
            Bare(..) | Flags => Some(quote!(write_bare)),
            Boxed(..) | TypeParameter(..) => Some(quote!(write_boxed)),
            ExtraDefault(..) => None,
            FlaggedTrue => Some(fail_hard()),
        }
    }

    fn is_unit(&self) -> bool {
        use self::WireKind::*;
        match *self {
            FlaggedTrue => true,
            _ => false,
        }
    }

    fn is_flags(&self) -> bool {
        use self::WireKind::*;
        match *self {
            Flags => true,
            _ => false,
        }
    }

    fn is_type_parameter(&self) -> bool {
        use self::WireKind::*;
        match *self {
            TypeParameter(..) => true,
            _ => false,
        }
    }

    fn is_extra(&self) -> bool {
        use self::WireKind::*;
        match *self {
            ExtraDefault(..) => true,
            _ => false,
        }
    }

    fn opt_names_slice(&self) -> Option<&[syn::Ident]> {
        use self::WireKind::*;
        match *self {
            Bare(ref t) |
            Boxed(ref t) |
            ExtraDefault(ref t) => t.idents.as_ref().map(|v| v.as_slice()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TypeIR {
    wire_kind: WireKind,
    needs_box: bool,
    needs_determiner: bool,
    with_option: bool,
}

impl TypeIR {
    fn from_names(names: &[String]) -> Self {
        Self::from_names_and_hint(names, false)
    }

    fn from_names_and_hint(names: &[String], force_bare: bool) -> Self {
        let wire_kind = WireKind::from_names_and_hint(names, force_bare);
        TypeIR {
            wire_kind, with_option: false,
            needs_box: match names.last().map(String::as_str) {
                // Special case two recursive types.
                Some("PageBlock") |
                Some("RichText") => true,
                _ => false,
            },
            needs_determiner: match names.last().map(String::as_str) {
                Some("vector") |
                Some("Vector") => true,
                _ => false,
            },
        }
    }

    fn bytes() -> Self {
        TypeIR {
            wire_kind: WireKind::Bare(["bytes"].iter().collect()),
            needs_box: false,
            needs_determiner: false,
            with_option: false,
        }
    }

    fn int() -> Self {
        TypeIR {
            wire_kind: WireKind::Bare(["int"].iter().collect()),
            needs_box: false,
            needs_determiner: false,
            with_option: false,
        }
    }

    fn type_parameter(id: syn::Ident) -> Self {
        TypeIR {
            wire_kind: WireKind::type_parameter(id),
            needs_box: false,
            needs_determiner: false,
            with_option: false,
        }
    }

    fn flags() -> Self {
        TypeIR {
            wire_kind: WireKind::Flags,
            needs_box: false,
            needs_determiner: false,
            with_option: false,
        }
    }

    fn repeated() -> Self {
        unimplemented!()
    }

    fn with_container(self, mut container: TypeIR) -> Self {
        container.wire_kind.become_container_for(container.needs_determiner, self.wire_kind);
        container
    }

    fn with_option_wrapper(mut self) -> Self {
        if !self.wire_kind.is_unit() {
            self.with_option = true;
        }
        self
    }

    fn io_turbofish(&self, with_tlobject: bool) -> Tokens {
        use self::WireKind::*;
        let mut ty = match self.wire_kind {
            Flags => quote!(crate::ton::Flags),
            TypeParameter(_) if with_tlobject => quote!(crate::ton::TLObject),
            _ => self.non_field_type(),
        };
        if self.needs_box {
            ty = quote!(Box<#ty>);
        }
        quote!(::<#ty>)
    }

    fn assemble_method(&self, with_tlobject: bool, method: Tokens) -> Tokens {
        let turbofish = self.io_turbofish(with_tlobject);
        quote!(#method #turbofish)
    }

    fn as_read_method(&self) -> Tokens {
        self.assemble_method(true, self.wire_kind.as_read_method())
    }

    fn as_write_method(&self) -> Option<Tokens> {
        self.wire_kind.as_write_method().map(|m| self.assemble_method(false, m))
    }

    fn non_field_type(&self) -> Tokens {
        use self::WireKind::*;
        match self.wire_kind {
            Bare(ref t) |
            Boxed(ref t) |
            ExtraDefault(ref t) => t.tokens.clone(),
            TypeParameter(ref t) => quote!(#t),
            _ => fail_hard(),
        }
    }

    fn unboxed(&self) -> Tokens {
        wrap_option_type(self.with_option, self.non_field_type())
    }

    fn boxed(&self) -> Tokens {
        let mut ty = self.non_field_type();
        if self.needs_box {
            ty = quote!(Box<#ty>);
        }
        wrap_option_type(self.with_option, ty)
    }

    fn field_type(&self) -> Tokens {
        if self.is_unit() {
            quote!(bool)
        } else {
            self.boxed()
        }
    }

    fn ref_prefix(&self) -> Tokens {
        if self.is_unit() {quote!()} else {quote!(ref)}
    }

    fn reference_prefix(&self) -> Tokens {
        if self.is_unit() {quote!()} else {quote!(&)}
    }

    fn local_reference_prefix(&self) -> Tokens {
        if self.is_unit() {quote!(&)} else {quote!()}
    }

    fn field_reference_type(&self) -> Tokens {
        let ref_ = self.reference_prefix();
        let ty = if self.is_unit() {
            quote!(bool)
        } else {
            self.non_field_type()
        };
        wrap_option_type(self.with_option, quote!(#ref_ #ty))
    }

    fn as_field_reference(&self, on: Tokens) -> Tokens {
        if self.is_unit() {
            wrap_option_value(self.with_option, quote!(#on))
        } else if self.with_option {
            quote!(#on.as_ref())
        } else {
            quote!(&#on)
        }
    }

    fn is_defined_trailer(&self) -> Tokens {
        use self::WireKind::*;
        match self.wire_kind {
            _ if self.with_option => quote!(.is_some()),
            FlaggedTrue => quote!(),
            _ => fail_hard(),
        }
    }

    fn is_unit(&self) -> bool {
        self.wire_kind.is_unit()
    }

    fn is_flags(&self) -> bool {
        self.wire_kind.is_flags()
    }

    fn is_type_parameter(&self) -> bool {
        self.wire_kind.is_type_parameter()
    }

    fn is_extra(&self) -> bool {
        self.wire_kind.is_extra()
    }

    fn owned_names_vec(&self) -> Vec<syn::Ident> {
        self.wire_kind.opt_names_slice()
            .unwrap()
            .iter()
            .cloned()
            .collect()
    }

    fn name(&self) -> syn::Ident {
        self.wire_kind.opt_names_slice()
            .and_then(|s| s.last())
            .unwrap()
            .clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FieldIR {
    name: String,
    ty: TypeIR,
    flag_bit: Option<(String, u32)>,
}

impl Field {
    fn resolved(&self, resolve_map: &TypeResolutionMap, replace_string_with_bytes: bool) -> FieldIR {
        let ty = if replace_string_with_bytes && self.ty.name() == Some("string") {
            TypeIR::bytes()
        } else {
            let is_flag_field = if let Some(ref value) = self.name {
                value == "flags"
            } else {
                false
            };
            self.ty.resolved(resolve_map, is_flag_field)
        };

        let name = if let Some(ref value) = self.name {
            value.clone()
        } else {
            String::new()
        };
        FieldIR { ty, name, flag_bit: self.ty.flag_field() }
    }
}

impl FieldIR {
    fn name(&self) -> syn::Ident {
        no_conflict_ident(&self.name)
    }

    fn local_name(&self) -> Option<syn::Ident> {
        no_conflict_local_ident(&self.name)
    }

    fn as_field(&self) -> Tokens {
        let name = self.name();
        let ty = self.ty.field_type();

        quote! {
            #name: #ty
        }
    }

    fn extra_default(field_name: &str, wire_kind_type: TypeName) -> Self {
        FieldIR {
            name: field_name.to_string(),
            ty: TypeIR {
                wire_kind: WireKind::ExtraDefault(wire_kind_type),
                needs_box: false,
                needs_determiner: false,
                with_option: false,
            },
            flag_bit: None,
        }
    }
}

impl Type {
    fn resolved(&self, resolve_map: &TypeResolutionMap, is_flag_field: bool) -> TypeIR {
        use Type::*;
        match *self {
            Named(ref names) => {
                match resolve_map.get(names) {
                    Some(ir) => return ir.clone(),
                    None => TypeIR::from_names(names),
                }
            },
            TypeParameter(ref name) => TypeIR::type_parameter(no_conflict_ident(name)),
            Generic(ref container, ref ty) => {
                let ty = ty.resolved(resolve_map, false);
                let container = match resolve_map.get(container) {
                    Some(ir) => ir.clone(),
                    None => TypeIR::from_names(container),
                };
                ty.with_container(container)
            },
            Flagged(_, _, ref ty) => {
                ty.resolved(resolve_map, false).with_option_wrapper()
            },
            Flags => TypeIR::flags(),
            Int if is_flag_field => TypeIR::flags(),
            Int => TypeIR::int(),
            Repeated(..) => TypeIR::repeated(),
        }
    }
}

impl Constructor<Type, Field> {
    fn resolve(self, which: Delimiter, resolve_map: &TypeResolutionMap) -> Constructor<TypeIR, FieldIR> {
        Constructor {
            variant: self.variant.resolved(resolve_map, false),
            fields: self.resolved_fields(resolve_map),
            output: self.resolved_output(which, resolve_map),
            type_parameters: self.type_parameters.iter()
                .map(|t| t.resolved(resolve_map, false))
                .collect(),
            tl_id: self.tl_id,
            original_variant: self.original_variant,
            original_output: self.original_output,
            is_function: self.is_function,
        }
    }

    fn resolved_output(&self, which: Delimiter, resolve_map: &TypeResolutionMap) -> TypeIR {
        if which == Delimiter::Functions && self.is_output_a_type_parameter() {
            TypeIR::type_parameter(no_conflict_ident(self.output.name().unwrap()))
        } else {
            self.output.resolved(resolve_map, false)
        }
    }

    fn is_output_a_type_parameter(&self) -> bool {
        let output_name = match &self.output {
            &Type::Named(ref v) if v.len() == 1 => v[0].as_str(),
            _ => return false,
        };
        for p in &self.type_parameters {
            if p.name.as_ref().map(String::as_str) == Some(output_name) {
                return true;
            }
        }
        false
    }

    fn resolved_fields(&self, resolve_map: &TypeResolutionMap) -> Vec<FieldIR> {
        let replace_string_with_bytes = match self.original_variant.as_str() {
            // types
            "resPQ" |
            "p_q_inner_data" |
            "p_q_inner_data_temp" |
            "server_DH_params_ok" |
            "server_DH_inner_data" |
            "client_DH_inner_data" |
            // functions
            "req_DH_params" |
            "set_client_DH_params" => true,
            _ => false,
        };

        let mut ret: Vec<_> = self.fields.iter()
            .map(|f| f.resolved(resolve_map, replace_string_with_bytes))
            .collect();
        if &self.original_variant == "manual.gzip_packed" {
            ret.push(FieldIR::extra_default(
                "unpacked",
                ["TLObject"].iter().collect::<TypeName>().transformed(|t| quote!(Option<#t>))));
        }
        ret
    }
}

impl Constructor<TypeIR, FieldIR> {
    fn fields_tokens(&self, pub_: Tokens, trailer: Tokens) -> Tokens {
        let pub_ = std::iter::repeat(pub_);
        if self.fields.is_empty() {
            quote! { #trailer }
        } else {
            let fields = self.fields.iter()
                .filter(|f| !f.ty.is_flags())
                .map(FieldIR::as_field);
            quote! {
                { #( #pub_ #fields , )* }
            }
        }
    }

    fn generics<F>(&self, mut param_cb: F) -> Tokens
        where F: FnMut(syn::Ident) -> Tokens,
    {
        if self.type_parameters.is_empty() {
            return quote!();
        }
        let bounds = self.type_parameters.iter().map(move |p| param_cb(p.name()));
        quote! { <#(#bounds),*> }
    }

    fn impl_generics(&self) -> Tokens {
        self.generics(|ty| quote!(#ty))
    }

    fn rpc_generics(&self) -> Tokens {
        self.generics(|ty| quote!(#ty: crate::Function))
    }

    fn deserialize_tlobject_generics(&self) -> Tokens {
        self.generics(|_| quote!(crate::ton::TLObject))
    }

    fn serialize_generics(&self) -> Tokens {
        self.generics(|ty| quote!(#ty: crate::AnyBoxedSerialize))
    }

    fn as_struct_determine_flags(&self, field_prefix: Tokens) -> Option<Tokens> {
        match self.fields.iter().filter(|f| f.ty.is_flags()).count() {
            0 => return None,
            1 => (),
            n => panic!("{} flags fields found on {:?}", n, self),
        }
        let determination = {
            let fields = self.fields.iter()
                .filter_map(|f| {
                    let name = f.name();
                    f.flag_bit.as_ref().map(|(_name, bit)| {
                        let is_defined = f.ty.is_defined_trailer();
                        quote! {
                            if #field_prefix #name #is_defined {
                                _flags |= 1 << #bit;
                            }
                        }
                    })
                });
            quote! {
                let mut _flags = 0u32;
                #( #fields )*
            }
        };
        Some(determination)
    }

    fn as_struct_doc(&self, matched: &str) -> String {
        format!("TL-derived from `{}`\n\n```text\n{}\n```\n", self.original_variant, matched)
    }

    fn as_struct_base(&self, name: &syn::Ident, matched: &str) -> Tokens {
        let doc = self.as_struct_doc(matched);
        let impl_generics = self.impl_generics();
        let fields = self.fields_tokens(quote! {pub}, quote! {;});
        quote! {
            #[derive(Debug, Default, Clone, PartialEq, Hash, Serialize, Deserialize)]
            #[doc = #doc]
            pub struct #name #impl_generics #fields
            impl Eq for #name {}
        }
    }

    fn as_struct_deserialize(&self) -> Tokens {
        if self.fields.is_empty() {
            return quote!(Ok(Self {}));
        }
        let mut names = Vec::<syn::Ident>::new();
        let mut reads = Vec::<Tokens>::new();
        for f in &self.fields {
            let read_method = f.ty.as_read_method();
            let expr = if let Some((ref flag_name, ref flag_bit)) = f.flag_bit {
                let flag_ident = no_conflict_ident(flag_name);
                let predicate = quote!(#flag_ident & (1 << #flag_bit) != 0);
                if f.ty.is_unit() {
                    predicate
                } else {
                    quote! {
                        if #predicate {
                            Some(_de. #read_method ()?)
                        } else {
                            None
                        }
                    }
                }
            } else {
                quote!(_de. #read_method ()?)
            };
            let name = f.name();
            if !f.ty.is_flags() {
                names.push(f.name());
            }
            reads.push(quote!(let #name = #expr;))
        }

        quote!({
            #( #reads )*
            Ok(Self { #( #names, )* })
        })
    }

    fn as_into_boxed(&self, name: &syn::Ident) -> Option<Tokens> {
        if self.tl_id().is_none() || self.is_function {
            return None;
        }
        let constructor = self.output.unboxed();
        let variant_name = self.full_variant_name();
        Some(quote! {
            impl crate::IntoBoxed for #name {
                type Boxed = #constructor;
                fn into_boxed(self) -> #constructor {
                    #constructor::#variant_name(Box::new(self))
                }
            }
        })
    }

    fn as_type_struct_base(&self, name: syn::Ident, matched: &str) -> Tokens {
        let serialize_destructure = self.as_variant_ref_destructure(&name)
            .map(|d| quote! { let &#d = self; })
            .unwrap_or_else(|| quote!());
        let serialize_stmts = self.as_variant_serialize();
        let deserialize = self.as_struct_deserialize();
        let type_impl = self.as_type_impl(
            &name,
            quote!(#serialize_destructure #serialize_stmts Ok(())),
            Some(quote!(#deserialize))
        );
        let struct_block = self.as_struct_base(&name, matched);
        let into_boxed = self.as_into_boxed(&name);
        quote! {
            #struct_block
            #type_impl
            #into_boxed
        }
    }

    fn variant_name(&self) -> syn::Ident {
        self.variant.name()
    }

    fn full_variant_name(&self) -> syn::Ident {
        let str_name = self.original_variant.clone();
        let name: String = str_name
            .split('.')
            .map(|s| {
                    let mut s = s.to_string();
                    s[..1].make_ascii_uppercase();
                    s
                })
            .collect::<Vec<String>>()
            .join("_");

        no_conflict_ident(&name)
    }

    fn as_variant_type_struct(&self, matched: &str) -> Tokens {
        if self.fields.is_empty() {
            quote!()
        } else {
            self.as_type_struct_base(self.variant_name(), matched)
        }
    }

    fn as_variant_ref_destructure(&self, name: &syn::Ident) -> Option<Tokens> {
        if self.fields.is_empty() {
            return None;
        }
        let fields = self.fields.iter()
            .filter(|f| !f.ty.is_flags())
            .map(|f| {
                let field_name = f.name();
                if f.ty.is_extra() {
                    return quote! { #field_name: _ }
                }
                let prefix = f.ty.ref_prefix();
                if let Some(local_name) = f.local_name() {
                    quote! { #field_name: #prefix #local_name }
                } else {
                    quote! { #prefix #field_name }
                }
            });
        Some(quote! {
            #name { #( #fields ),* }
        })
    }

    fn as_variant_serialize(&self) -> Tokens {
        let determine_flags = self.as_struct_determine_flags(quote!())
            .unwrap_or_else(|| quote!());
        let fields = self.fields.iter()
            .map(|f| {
                if f.ty.is_unit() {
                    return quote!();
                }
                let write_method = match f.ty.as_write_method() {
                    Some(m) => m,
                    None => return quote!(),
                };
                let field_name = f.name();
                let local_name = f.local_name().unwrap_or_else(|| field_name.clone());
                if f.ty.is_flags() {
                    quote! { _ser. #write_method (&_flags)?; }
                } else if f.flag_bit.is_some() {
                    let outer_ref = f.ty.reference_prefix();
                    let inner_ref = f.ty.ref_prefix();
                    let local_ref = f.ty.local_reference_prefix();
                    quote! {
                        if let #outer_ref Some(#inner_ref inner) = #local_name {
                            _ser. #write_method (#local_ref inner)?;
                        }
                    }
                } else {
                    let prefix = f.ty.local_reference_prefix();
                    quote! { _ser. #write_method (#prefix #local_name)?; }
                }
            });
        quote! {
            #determine_flags
            #( #fields )*
        }
    }

    fn as_function_struct(&self, matched: &str) -> Tokens {
        let name = self.variant_name();
        let tl_id = self.tl_id().unwrap();
        let rpc_generics = self.rpc_generics();
        let deserialize_generics = self.deserialize_tlobject_generics();
        let serialize_generics = self.serialize_generics();
        let impl_generics = self.impl_generics();
        let mut output_ty = self.output.boxed();
        if self.output.is_type_parameter() {
            output_ty = quote! {#output_ty::Reply};
        }
        let base = self.as_type_struct_base(self.variant_name(), matched);

        quote! {
            #base

            impl crate::BoxedDeserialize for #name #deserialize_generics {
                fn possible_constructors() -> Vec<crate::ConstructorNumber> { vec![#tl_id] }
                fn deserialize_boxed(id: crate::ConstructorNumber, de: &mut crate::Deserializer) -> crate::Result<Self> {
                    if id == #tl_id {
                        de.read_bare()
                    } else {
                        _invalid_id!(id)
                    }
                }
            }

            impl #serialize_generics crate::BoxedSerialize for #name #impl_generics {
                fn serialize_boxed(&self) -> (crate::ConstructorNumber, &crate::BareSerialize) {
                    (#tl_id, self)
                }
            }

            impl #rpc_generics crate::Function for #name #impl_generics {
                type Reply = #output_ty;
            }
        }
    }

    fn as_variant(&self) -> Tokens {
        let name = self.full_variant_name();
        if self.fields.is_empty() {
            quote!(#name)
        } else {
            let type_name = self.variant.unboxed();
            quote!(#name(Box<#type_name>))
        }
    }

    fn as_variant_serialize_arm(&self) -> Tokens {
        let tl_id = self.tl_id().unwrap();
        if self.fields.is_empty() {
            quote!(=> (#tl_id, &()))
        } else {
            quote!((ref x) => (#tl_id, x.as_ref()))
        }
    }

    fn as_variant_deserialize(&self, wrap_to_box: bool) -> Tokens {
        if self.fields.is_empty() {
            quote!()
        } else {
            let read_method = self.variant.as_read_method();
            if wrap_to_box {
                quote!((Box::new(_de. #read_method ()?)))
            } else {
                quote!((_de. #read_method ()?))
            }
        }
    }

    fn tl_id(&self) -> Option<Tokens> {
        self.tl_id.as_ref().map(|tl_id| {
            let tl_id: syn::LitInt = syn::parse_str(&format!("0x{:08x}", tl_id)).unwrap();
            quote!(crate::ConstructorNumber(#tl_id))
        })
    }

    fn as_type_impl(&self, name: &syn::Ident, serialize: Tokens, deserialize: Option<Tokens>) -> Tokens {
        let serialize_generics = self.serialize_generics();
        let impl_generics = self.impl_generics();

        let deserialize = deserialize.map(|body| {
            let deserialize_generics = self.deserialize_tlobject_generics();
            quote! {
                impl crate::BareDeserialize for #name #deserialize_generics {
                    fn deserialize_bare(_de: &mut crate::Deserializer) -> crate::Result<Self> {
                        #body
                    }
                }
            }
        }).unwrap_or_else(|| quote!());

        quote! {
            impl #serialize_generics crate::BareSerialize for #name #impl_generics {
                fn serialize_bare(&self, _ser: &mut crate::Serializer) -> crate::Result<()> {
                    #serialize
                }
            }

            #deserialize
        }
    }

    fn as_dynamic_deserializer(&self) -> Option<Tokens> {
        let tl_id = self.tl_id()?;
        let type_name = if self.is_function {
            Cow::Owned(format!("rpc.{}", self.original_variant))
        } else {
            Cow::Borrowed(&self.original_variant)
        };
        let mut ty = if &self.original_variant == "manual.gzip_packed" {
            quote!(crate::ton::TransparentGunzip)
        } else if self.is_function {
            self.variant.unboxed()
        } else {
            self.output.unboxed()
        };
        if !self.type_parameters.is_empty() {
            let generics = self.type_parameters.iter().map(|_| quote!(crate::ton::TLObject));
            ty = quote!(#ty<#(#generics),*>);
        }
        Some(quote! {
            crate::DynamicDeserializer::from::<#ty>(#tl_id, #type_name)
        })
    }
}

fn camelize<F>(resolve_map: &mut TypeResolutionMap, ty: &mut Type, additional_correction: F) -> NameChunks
    where F: FnOnce(&mut NameChunks, &mut Vec<String>) -> bool
{
    let names = ty.names_vec_mut().unwrap();
    let fixup_key = names.clone();
    let name = names.pop().unwrap();
    let mut new_name = NameChunks::from_name(&name).unwrap();
    let force_bare = additional_correction(&mut new_name, names);
    names.push(new_name.as_upper_camel_case());
    let type_ir = TypeIR::from_names_and_hint(&names, force_bare);
    resolve_map.insert(fixup_key, type_ir.clone());
    resolve_map.insert(names.clone(), type_ir);
    new_name
}

impl<Ty, Fi> Constructors<Ty, Fi> {
    fn first_constructor(&self) -> &Constructor<Ty, Fi> {
        &self.0[0].0
    }
}

impl Constructors<Type, Field> {
    fn resolve(self, resolve_map: &TypeResolutionMap) -> Constructors<TypeIR, FieldIR> {
        Constructors({
            self.0.into_iter()
                .map(|Matched(c, m)| Matched(c.resolve(Delimiter::Types, resolve_map), m))
                .collect()
        })
    }

    fn fix_names(&mut self, base_ns: &[String], resolve_map: &mut TypeResolutionMap) {
        let output_name = camelize(resolve_map, &mut self.0[0].0.output, |_, _| false);

        let common_prefix = self.0.iter()
            .filter_map(|m| m.0.variant.name())
            .map(|n| NameChunks::from_name(n).unwrap())
            .fold(None, |a_opt: Option<NameChunks>, b| {
                Some(a_opt.map(|a| a.common_prefix_of(&b)).unwrap_or(b))
            })
            .unwrap_or_else(|| NameChunks(vec![]));
        let common_module = if common_prefix.0.is_empty() {
            output_name.as_snake_case()
        } else {
            common_prefix.as_snake_case()
        };
        for &mut Matched(ref mut c, _) in &mut self.0 {
            camelize(resolve_map, &mut c.variant, |name, names| {
                if !names.starts_with(base_ns) {
                    names.splice(..0, base_ns.iter().cloned());
                }
                names.push(common_module.clone());
                let was_bare = is_first_char_lowercase(&name.0[0]);
                name.trim_common_prefix(&common_prefix);
                if name.0.is_empty() {
                    name.clone_from(&common_prefix);
                }
                was_bare
            });
        }
    }

}

impl Constructors<TypeIR, FieldIR> {
    fn coalesce_methods(&self) -> BTreeMap<&str, BTreeMap<&TypeIR, BTreeSet<&Constructor<TypeIR, FieldIR>>>> {
        let mut map: BTreeMap<&str, BTreeMap<&TypeIR, BTreeSet<&Constructor<TypeIR, FieldIR>>>> = BTreeMap::new();
        for &Matched(ref cons, _) in &self.0 {
            for field in &cons.fields {
                if field.ty.is_flags() {
                    continue
                }
                map.entry(&field.name)
                    .or_insert_with(Default::default)
                    .entry(&field.ty)
                    .or_insert_with(Default::default)
                    .insert(cons);
            }
        }
        map
    }

    fn as_only_unwrap(&self, enum_name: &syn::Ident) -> Option<Tokens> {
        let &Matched(ref cons, _) = self.0.first()?;
        if self.0.len() != 1 || cons.fields.is_empty() {
            return None
        }
        let cons_name = cons.full_variant_name();
        let ty = cons.variant.unboxed();
        Some(quote! {
            pub fn only(self) -> #ty {
                match self {
                    #enum_name::#cons_name(x) => *x
                }
            }
        })
    }

    fn determine_methods(&self, enum_name: &syn::Ident) -> Tokens {
        let all_constructors = self.0.len();
        let mut methods = vec![];
        for (name, typemap) in self.coalesce_methods() {
            if typemap.len() != 1 {
                continue;
            }
            let name = no_conflict_ident(name);
            let (ty_ir, constructors) = typemap.into_iter().next().unwrap();
            let mut return_ir = ty_ir.clone();
            let exhaustive = constructors.len() == all_constructors;
            if !exhaustive {
                return_ir.with_option = true;
            }
            let value = wrap_option_value(!exhaustive && !ty_ir.with_option, ty_ir.as_field_reference(quote!(x.#name)));
            let constructors = constructors.into_iter()
                .map(|c| {
                    let cons_name = c.full_variant_name();
                    quote!(&#enum_name::#cons_name(ref x) => #value)
                });
            let trailer = if exhaustive {
                quote!()
            } else {
                quote!(_ => None)
            };
            let ty = return_ir.field_reference_type();
            methods.push(quote! {
                pub fn #name(&self) -> #ty {
                    match self {
                        #( #constructors, )*
                        #trailer
                    }
                }
            });
        }

        methods.extend(self.as_only_unwrap(enum_name));

        if methods.is_empty() {
            quote!()
        } else {
            quote! {
                impl #enum_name {
                    #( #methods )*
                }
            }
        }
    }

    fn as_type_impl(&self, name: &syn::Ident, serialize: Tokens, deserialize: Tokens) -> Tokens {
        let tl_ids = self.as_tl_ids();

        quote! {

            impl crate::BoxedSerialize for #name {
                fn serialize_boxed(&self) -> (crate::ConstructorNumber, &crate::BareSerialize) {
                    #serialize
                }
            }

            impl crate::BoxedDeserialize for #name {
                fn possible_constructors() -> Vec<crate::ConstructorNumber> { vec![#tl_ids] }
                fn deserialize_boxed(_id: crate::ConstructorNumber, _de: &mut crate::Deserializer) -> crate::Result<Self> {
                    #deserialize
                }
            }

        }
    }

    fn as_option_type_impl(&self) -> Tokens {
        if self.0.len() != 2 {
            return quote!();
        }
        let tl_ids = self.constructors_and_tl_ids().collect::<Vec<_>>();
        let empty = tl_ids.iter().find(|&&(_, c)| c.fields.is_empty());
        let nonempty = tl_ids.iter().find(|&&(_, c)| !c.fields.is_empty());
        let (empty_id, nonempty_id, nonempty_cons) = match (empty, nonempty) {
            (Some(&(ref i_e, _)), Some(&(ref i_n, c_n))) => (i_e, i_n, c_n),
            _ => return quote!(),
        };
        let nonempty_variant = nonempty_cons.variant.unboxed();
        let nonempty_deserialize = nonempty_cons.as_variant_deserialize(false);

        quote! {

            impl crate::BoxedSerialize for Option<#nonempty_variant> {
                fn serialize_boxed(&self) -> (crate::ConstructorNumber, &crate::BareSerialize) {
                    match *self {
                        None => (#empty_id, &()),
                        Some(ref x) => (#nonempty_id, x),
                    }
                }
            }

            impl crate::BoxedDeserialize for Option<#nonempty_variant> {
                fn possible_constructors() -> Vec<crate::ConstructorNumber> { vec![#empty_id, #nonempty_id] }
                fn deserialize_boxed(_id: crate::ConstructorNumber, _de: &mut crate::Deserializer) -> crate::Result<Self> {
                    match _id {
                        #empty_id => Ok(None),
                        #nonempty_id => Ok(Some #nonempty_deserialize),
                        id => _invalid_id!(id),
                    }
                }
            }

        }
    }

    fn constructors_and_tl_ids<'this>(&'this self) -> Box<dyn 'this + Iterator<Item = (Tokens, &'this Constructor<TypeIR, FieldIR>)>> {
        Box::new(self.0.iter().filter_map(|cm| {
            cm.0.tl_id().map(|id| (id, &cm.0))
        }))
    }

    fn as_serialize_match(&self, enum_name: &syn::Ident) -> Tokens {
        let constructors = self.0.iter()
            .map(|&Matched(ref c, _)| {
                let variant_name = c.full_variant_name();
                let serialize = c.as_variant_serialize_arm();
                quote!(&#enum_name::#variant_name #serialize)
            });
        quote! {
            match self {
                #( #constructors, )*
            }
        }
    }

    fn as_tl_ids(&self) -> Tokens {
        let tl_ids = self.0.iter()
            .filter_map(|cm| cm.0.tl_id());
        quote!(#( #tl_ids, )*)
    }

    fn as_deserialize_match(&self, enum_name: &syn::Ident) -> Tokens {
        let constructors = self.constructors_and_tl_ids()
            .map(|(tl_id, c)| {
                let variant_name = c.full_variant_name();
                let deserialize = c.as_variant_deserialize(true);
                quote!(#tl_id => Ok(#enum_name::#variant_name #deserialize))
            });
        quote! {
            match _id {
                #( #constructors, )*
                id => _invalid_id!(id),
            }
        }
    }

    fn as_enum_doc(&self) -> String {
        use std::fmt::Write;
        let mut ret = format!("TL-derived from `{}`\n\n```text\n", &self.first_constructor().original_output);
        for (e, cm) in self.0.iter().enumerate() {
            if e != 0 {
                ret.write_str("\n\n").unwrap();
            }
            ret.write_str(&cm.1).unwrap();
        }
        write!(ret, "\n```\n").unwrap();
        ret
    }

    fn enum_default_impl(&self) -> Option<Tokens> {

        let &Matched(ref cons, _) = self.0.first()?;
        let name = self.first_constructor().output.name();
        let variant = cons.full_variant_name();

        if cons.fields.is_empty() {
            return Some(quote! {
                impl Default for #name {
                    fn default() -> Self {
                        #name::#variant
                    }
                }
            })
        }
        let name = self.first_constructor().output.name();
        let variant = cons.full_variant_name();
        
        let ty = cons.variant.unboxed();

        Some(quote! {
            impl Default for #name {
                fn default() -> Self {
                    #name::#variant(Box::new(#ty::default()))
                }
            }
        })
    }

    fn as_enum(&self) -> Tokens {
        if self.0.iter().all(|cm| cm.0.tl_id().is_none()) {
            return quote!();
        }
        let name = self.first_constructor().output.name();
        let doc = self.as_enum_doc();
        let variants = self.0.iter()
            .map(|cm| cm.0.as_variant());
        let methods = self.determine_methods(&name);
        let type_impl = self.as_type_impl(
            &name,
            self.as_serialize_match(&name),
            self.as_deserialize_match(&name));
        let option_type_impl = self.as_option_type_impl();
        let default_impl = self.enum_default_impl();

        quote! {
            #[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
            #[doc = #doc]
            pub enum #name {
                #( #variants , )*
            }
            #methods
            impl Eq for #name {}
            #default_impl
            #type_impl
            #option_type_impl
        }
    }
}

pub fn generate_code_for(input: &str, path: &Path) {
    let constructors = {
        let mut items = parser::parse_string(input).unwrap();
        filter_items(&mut items);
        AllConstructors::from_matched_items(items)
    };

    let layer = constructors.layer as i32;
    let prelude = quote! {
        #![allow(bare_trait_objects, unused_variables, unused_imports, non_snake_case)]
        pub use crate::ton_prelude::*;

        use serde_derive::{Serialize, Deserialize};

        pub const LAYER: i32 = #layer;
    }.to_string();

    constructors.print_tokens(Some(prelude), path);
}
