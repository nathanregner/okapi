use crate::get_add_operation_fn_name;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::parse::{Parse, ParseBuffer};
use syn::{punctuated::Punctuated, token::Comma, Path, Result};

struct Routes {
    modify_spec: Option<Path>,
    routes: Punctuated<Path, Comma>,
}

mod kw {
    syn::custom_keyword!(spec);
}

impl Parse for Routes {
    fn parse(input: &ParseBuffer<'_>) -> Result<Self> {
        let modify_spec = if input.peek(kw::spec) {
            input.parse::<kw::spec>()?;
            input.parse::<Token![:]>()?;
            let path = input.parse()?;
            input.parse::<Token![;]>()?;
            Some(path)
        } else {
            None
        };
        let routes = input.parse_terminated(Path::parse)?;
        Ok(Self {
            modify_spec,
            routes,
        })
    }
}

pub fn parse(routes: TokenStream) -> TokenStream {
    let routes = parse_macro_input!(routes as Routes);
    parse_inner(routes)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn parse_inner(routes: Routes) -> Result<TokenStream2> {
    let paths = routes.routes;
    let add_operations = create_add_operations(paths.clone())?;
    let modify_spec = routes.modify_spec;
    Ok(quote! {
        {
            let settings = ::rocket_okapi::settings::OpenApiSettings::new();
            let mut gen = ::rocket_okapi::gen::OpenApiGenerator::new(settings.clone());
            #add_operations
            let mut spec = gen.into_openapi();
            let mut info = ::okapi::openapi3::Info {
                title: env!("CARGO_PKG_NAME").to_owned(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
                ..Default::default()
            };
            if !env!("CARGO_PKG_DESCRIPTION").is_empty() {
                info.description = Some(env!("CARGO_PKG_DESCRIPTION").to_owned());
            }
            if !env!("CARGO_PKG_REPOSITORY").is_empty() {
                info.contact = Some(::okapi::openapi3::Contact{
                    name: Some("Repository".to_owned()),
                    url: Some(env!("CARGO_PKG_REPOSITORY").to_owned()),
                    ..Default::default()
                });
            }
            if !env!("CARGO_PKG_HOMEPAGE").is_empty() {
                info.contact = Some(::okapi::openapi3::Contact{
                    name: Some("Homepage".to_owned()),
                    url: Some(env!("CARGO_PKG_REPOSITORY").to_owned()),
                    ..Default::default()
                });
            }
            spec.info = info;
            #modify_spec(&mut spec);

            let mut routes = ::rocket::routes![#paths];
            routes.push(::rocket_okapi::handlers::OpenApiHandler::new(spec).into_route(&settings.json_path));
            routes
        }
    })
}

fn create_add_operations(paths: Punctuated<Path, Comma>) -> Result<TokenStream2> {
    let function_calls = paths.into_iter().map(|path| {
        let fn_name = fn_name_for_add_operation(path.clone());
        let operation_id = operation_id(&path);
        quote! {
            #fn_name(&mut gen, #operation_id.to_owned())
                .expect(&format!("Could not generate OpenAPI operation for `{}`.", stringify!(#path)));
        }
    });
    Ok(quote! {
        #(#function_calls)*
    })
}

fn fn_name_for_add_operation(mut fn_path: Path) -> Path {
    let mut last_seg = fn_path.segments.last_mut().expect("syn::Path has segments");
    last_seg.ident = get_add_operation_fn_name(&last_seg.ident);
    fn_path
}

fn operation_id(fn_path: &Path) -> String {
    let idents: Vec<String> = fn_path
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect();
    idents.join("_")
}
