use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::Arm;
use syn::FnArg;
use syn::Ident;
use syn::ImplItem;
use syn::ItemImpl;
use syn::PatType;
use syn::ReturnType;
use syn::Type;
use syn::parse_macro_input;
use syn::parse_quote;

#[doc(hidden)]
#[proc_macro_attribute]
pub fn instruction_executor(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemImpl);

    let arms = input
        .items
        .iter_mut()
        .filter_map(|item| match item {
            ImplItem::Fn(func) => Some(func),
            _ => None,
        })
        .map(|func| -> Arm {
            if func.sig.inputs.len() == 1 {
                func.sig.inputs.push(parse_quote! {
                    _arg: Option<u16>
                });
            } else if func.sig.inputs.len() != 2 {
                panic!("Invalid instruction function");
            }

            if let Some(FnArg::Typed(PatType { ty, pat, .. })) = func.sig.inputs.last_mut()
                && let Type::Path(path) = ty.as_ref()
                && path.qself.is_none()
                && path.path.is_ident("u16")
            {
                *ty = parse_quote!(::std::option::Option<u16>);

                let name = func.sig.ident.to_string().to_uppercase();
                let stmts = std::mem::take(&mut func.block.stmts);

                func.block.stmts = parse_quote! {
                    let #pat = #pat.expect(concat!("Invalid argument for ", #name));
                    #(#stmts)*
                }
            }

            if matches!(func.sig.output, ReturnType::Default) {
                func.sig.output = parse_quote! { -> crate::error::Result<()> };

                // Wrapping the function's statements in a lambda prevents question mark use.
                let stmts = std::mem::take(&mut func.block.stmts);
                func.block.stmts.push(parse_quote! {
                    (|| {
                        #(#stmts)*
                    })();
                });

                func.block.stmts.push(parse_quote! { return Ok(()); });
            }

            let ident = &func.sig.ident;

            let instruction = {
                let string = ident.to_string();
                let mut chars = string.chars();
                let name = chars.next().unwrap().to_uppercase().to_string() + chars.as_str();

                Ident::new(&name, Span::call_site())
            };

            parse_quote! {
                crate::cpu::Instruction::#instruction => Self::#ident
            }
        })
        .collect::<Vec<_>>();

    input.items.push(parse_quote! {
        fn execute(&mut self, instruction: crate::cpu::Instruction, arg: Option<u16>) -> crate::error::Result<()> {
            let func = match instruction {
                #(#arms),*,
                _ => panic!("Unimplemented instruction in executor")
            };

            func(self, arg)
        }
    });

    TokenStream::from(quote!(#input))
}
