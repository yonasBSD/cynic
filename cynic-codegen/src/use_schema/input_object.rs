use {
    quote::{quote, ToTokens, TokenStreamExt},
    syn::parse_quote,
};

use crate::schema::types::{InputObjectType, InputValue};

pub struct InputObjectOutput<'a> {
    object: InputObjectType<'a>,
    object_marker: proc_macro2::Ident,
}

struct FieldOutput<'a> {
    field: &'a InputValue<'a>,
    object_marker: &'a proc_macro2::Ident,
}

impl<'a> InputObjectOutput<'a> {
    pub fn new(object: InputObjectType<'a>) -> Self {
        InputObjectOutput {
            object_marker: object.marker_ident().to_rust_ident(),
            object,
        }
    }

    pub fn append_fields(&self, field_module: &mut proc_macro2::TokenStream) {
        if !self.object.fields.is_empty() {
            let field_module_ident = self.object.field_module().ident();
            let fields = self.object.fields.iter().map(|f| FieldOutput {
                field: f,
                object_marker: &self.object_marker,
            });
            field_module.append_all(quote! {
                pub mod #field_module_ident {
                    #(#fields)*
                }
            });
        }
    }
}

impl ToTokens for InputObjectOutput<'_> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let object_marker = &self.object_marker;
        tokens.append_all(quote! {
            pub struct #object_marker;

            impl cynic::schema::InputObjectMarker for #object_marker {}
        });
    }
}

impl ToTokens for FieldOutput<'_> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let object_marker = self.object_marker;
        let field_marker = &self.field.marker_ident().to_rust_ident();
        let field_name_literal = proc_macro2::Literal::string(self.field.name.as_str());

        let field_type_marker = self
            .field
            .value_type
            .marker_type()
            .to_path(&parse_quote! { super::super });

        tokens.append_all(quote! {
            pub struct #field_marker;

            impl cynic::schema::Field for #field_marker {
                type Type = #field_type_marker;

                const NAME: &'static ::core::primitive::str = #field_name_literal;
            }

            impl cynic::schema::HasInputField<#field_marker, #field_type_marker> for super::super::#object_marker {
            }
        });
    }
}
