mod builtins;
mod fields;
mod type_index;
mod type_refs;

pub(crate) use builtins::add_builtins;
pub use fields::*;

pub use type_index::{GraphPath, TypeIndex};
pub use type_refs::{InputTypeRef, InterfaceTypeRef, OutputTypeRef, TypeRef};

use cynic_parser::type_system as parser;

use std::{convert::TryFrom, iter, rc::Rc};

use crate::Error;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Type<'schema> {
    Scalar(ScalarDetails<'schema>),
    Object(ObjectDetails<'schema>),
    Interface(InterfaceDetails<'schema>),
    Enum(EnumDetails<'schema>),
    Union(UnionDetails<'schema>),
    InputObject(InputObjectDetails<'schema>),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct ScalarDetails<'schema> {
    pub name: &'schema str,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectDetails<'schema> {
    pub name: &'schema str,
    pub fields: Vec<OutputField<'schema>>,
    implements_interfaces: Vec<InterfaceTypeRef<'schema>>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InterfaceDetails<'schema> {
    pub name: &'schema str,
    pub fields: Vec<OutputField<'schema>>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct EnumDetails<'schema> {
    pub name: &'schema str,
    pub values: Vec<&'schema str>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnionDetails<'schema> {
    pub name: &'schema str,
    pub types: Vec<OutputTypeRef<'schema>>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct InputObjectDetails<'schema> {
    pub name: &'schema str,
    pub fields: Vec<InputField<'schema>>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum InputType<'schema> {
    Scalar(ScalarDetails<'schema>),
    Enum(EnumDetails<'schema>),
    InputObject(InputObjectDetails<'schema>),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OutputType<'schema> {
    Scalar(ScalarDetails<'schema>),
    Object(ObjectDetails<'schema>),
    Interface(InterfaceDetails<'schema>),
    Enum(EnumDetails<'schema>),
    Union(UnionDetails<'schema>),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InterfaceType<'schema>(pub InterfaceDetails<'schema>);

impl<'schema> Type<'schema> {
    fn from_type_definition(
        type_def: &parser::TypeDefinition<'schema>,
        type_index: &Rc<TypeIndex<'schema>>,
    ) -> Type<'schema> {
        use parser::TypeDefinition;
        let typename_field = type_index.typename_field();

        match type_def {
            TypeDefinition::Scalar(scalar) => Type::Scalar(ScalarDetails {
                name: scalar.name(),
            }),
            TypeDefinition::Object(obj) => Type::Object(ObjectDetails {
                name: obj.name(),
                fields: obj
                    .fields()
                    .chain(iter::once(typename_field))
                    .map(|field| OutputField::from_parser(field, type_index))
                    .collect(),
                implements_interfaces: obj
                    .implements_interfaces()
                    .map(|name| InterfaceTypeRef::new(name, type_index))
                    .collect(),
            }),
            TypeDefinition::Interface(iface) => Type::Interface(InterfaceDetails {
                name: iface.name(),
                fields: iface
                    .fields()
                    .chain(iter::once(typename_field))
                    .map(|field| OutputField::from_parser(field, type_index))
                    .collect(),
            }),
            TypeDefinition::Union(union) => Type::Union(UnionDetails {
                name: union.name(),
                types: union
                    .members()
                    .map(|member| OutputTypeRef::new(member.name(), type_index))
                    .collect(),
            }),
            TypeDefinition::Enum(def) => Type::Enum(EnumDetails {
                name: def.name(),
                values: def.values().map(|v| v.value()).collect(),
            }),
            TypeDefinition::InputObject(obj) => Type::InputObject(InputObjectDetails {
                name: obj.name(),
                fields: obj
                    .fields()
                    .map(|field| InputField::from_parser(field, type_index))
                    .collect(),
            }),
        }
    }

    pub fn name(&self) -> &'schema str {
        match self {
            Self::Scalar(details) => details.name,
            Self::Object(details) => details.name,
            Self::Interface(details) => details.name,
            Self::Enum(details) => details.name,
            Self::Union(details) => details.name,
            Self::InputObject(details) => details.name,
        }
    }

    pub fn allows_fragment_target_of(&self, target: &Type<'schema>) -> Result<(), Error> {
        if self == target {
            return Ok(());
        }

        match self {
            Type::Interface(iface) => {
                if let Type::Object(obj) = target {
                    if obj.implements_interface(iface) {
                        return Ok(());
                    }
                }
                Err(Error::TypeDoesNotImplementInterface(
                    target.name().to_string(),
                    iface.name.to_string(),
                ))
            }
            Type::Union(details) => {
                if details.has_member(target) {
                    return Ok(());
                }

                Err(Error::TypeNotUnionMember(
                    target.name().to_string(),
                    self.name().to_string(),
                ))
            }
            Type::Object(obj) => {
                // If current context is an object, we allow spreads of:
                // The current object, or an interface/union the current object
                // implements/is a member of.
                match target {
                    Type::Interface(iface) => {
                        if obj.implements_interface(iface) {
                            return Ok(());
                        }

                        Err(Error::TypeDoesNotImplementInterface(
                            target.name().to_string(),
                            iface.name.to_string(),
                        ))
                    }
                    Type::Union(union_details) => {
                        if union_details.has_member(self) {
                            return Ok(());
                        }

                        Err(Error::TypeNotUnionMember(
                            target.name().to_string(),
                            self.name().to_string(),
                        ))
                    }
                    _ => Err(Error::TypeConditionFailed(
                        target.name().to_string(),
                        self.name().to_string(),
                    )),
                }
            }
            _ => Err(Error::TypeConditionFailed(
                target.name().to_string(),
                self.name().to_string(),
            )),
        }
    }
}

impl ScalarDetails<'_> {
    pub fn is_builtin(&self) -> bool {
        matches!(self.name, "String" | "Int" | "Boolean" | "ID" | "Float")
    }
}

impl<'schema> ObjectDetails<'schema> {
    fn implements_interface(&self, interface: &InterfaceDetails<'schema>) -> bool {
        self.implements_interfaces
            .iter()
            .any(|iface_ref| iface_ref.lookup().ok().map(|i| i.0.name) == Some(interface.name))
    }
}

impl<'schema> UnionDetails<'schema> {
    fn has_member(&self, member: &Type<'schema>) -> bool {
        self.types
            .iter()
            .any(|type_ref| type_ref.lookup().ok().map(|t| t.name()) == Some(member.name()))
    }
}

impl<'schema> OutputType<'schema> {
    pub fn name(&self) -> &'schema str {
        match self {
            Self::Scalar(details) => details.name,
            Self::Object(details) => details.name,
            Self::Interface(details) => details.name,
            Self::Enum(details) => details.name,
            Self::Union(details) => details.name,
        }
    }
}

impl<'schema> TryFrom<Type<'schema>> for InputType<'schema> {
    type Error = Error;

    fn try_from(ty: Type<'schema>) -> Result<InputType<'schema>, Error> {
        match ty {
            Type::InputObject(inner) => Ok(InputType::InputObject(inner)),
            Type::Scalar(inner) => Ok(InputType::Scalar(inner)),
            Type::Enum(inner) => Ok(InputType::Enum(inner)),
            _ => Err(Error::ExpectedInputType),
        }
    }
}

impl<'schema> TryFrom<Type<'schema>> for OutputType<'schema> {
    type Error = Error;

    fn try_from(ty: Type<'schema>) -> Result<OutputType<'schema>, Error> {
        match ty {
            Type::Scalar(inner) => Ok(OutputType::Scalar(inner)),
            Type::Enum(inner) => Ok(OutputType::Enum(inner)),
            Type::Object(inner) => Ok(OutputType::Object(inner)),
            Type::Interface(inner) => Ok(OutputType::Interface(inner)),
            Type::Union(inner) => Ok(OutputType::Union(inner)),
            Type::InputObject(_) => Err(Error::ExpectedOutputType),
        }
    }
}

impl<'schema> TryFrom<Type<'schema>> for InterfaceType<'schema> {
    type Error = Error;

    fn try_from(ty: Type<'schema>) -> Result<InterfaceType<'schema>, Error> {
        match ty {
            Type::Interface(inner) => Ok(InterfaceType(inner)),
            _ => Err(Error::ExpectedInterfaceType),
        }
    }
}
