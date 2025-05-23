use super::prelude::*;
use super::{
    descriptions::Description,
    directives::Directive,
    ids::{DescriptionId, DirectiveId, ScalarDefinitionId},
    TypeSystemId,
};
#[allow(unused_imports)]
use std::fmt::{self, Write};

pub struct ScalarDefinitionRecord {
    pub name: StringId,
    pub name_span: Span,
    pub description: Option<DescriptionId>,
    pub directives: IdRange<DirectiveId>,
    pub span: Span,
}

#[derive(Clone, Copy)]
pub struct ScalarDefinition<'a>(pub(in super::super) ReadContext<'a, ScalarDefinitionId>);

impl<'a> ScalarDefinition<'a> {
    pub fn name(&self) -> &'a str {
        let document = &self.0.document;
        document.lookup(document.lookup(self.0.id).name)
    }
    pub fn name_span(&self) -> Span {
        let document = self.0.document;
        document.lookup(self.0.id).name_span
    }
    pub fn description(&self) -> Option<Description<'a>> {
        let document = self.0.document;
        document
            .lookup(self.0.id)
            .description
            .map(|id| document.read(id))
    }
    pub fn directives(&self) -> Iter<'a, Directive<'a>> {
        let document = self.0.document;
        super::Iter::new(document.lookup(self.0.id).directives, document)
    }
    pub fn span(&self) -> Span {
        let document = self.0.document;
        document.lookup(self.0.id).span
    }
}

impl ScalarDefinition<'_> {
    pub fn id(&self) -> ScalarDefinitionId {
        self.0.id
    }
}

impl fmt::Debug for ScalarDefinition<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScalarDefinition")
            .field("name", &self.name())
            .field("description", &self.description())
            .field("directives", &self.directives())
            .field("span", &self.span())
            .finish()
    }
}

impl TypeSystemId for ScalarDefinitionId {
    type Reader<'a> = ScalarDefinition<'a>;
    fn read(self, document: &TypeSystemDocument) -> Self::Reader<'_> {
        ScalarDefinition(ReadContext { id: self, document })
    }
}

impl IdReader for ScalarDefinition<'_> {
    type Id = ScalarDefinitionId;
    type Reader<'a> = ScalarDefinition<'a>;
    fn new(id: Self::Id, document: &'_ TypeSystemDocument) -> Self::Reader<'_> {
        document.read(id)
    }
}
