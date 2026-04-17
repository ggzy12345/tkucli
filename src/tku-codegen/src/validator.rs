use std::collections::HashSet;
use tku_core::schema::{AppSchema, ArgType, ResourceSchema};

pub struct SchemaValidator<'a> {
    schema: &'a AppSchema,
}

impl<'a> SchemaValidator<'a> {
    pub fn new(schema: &'a AppSchema) -> Self {
        Self { schema }
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        self.check_root_verb_uniqueness()?;
        self.check_root_vs_resource_collisions()?;
        self.check_resource_name_uniqueness(&self.schema.resources, None)?;
        self.check_verb_uniqueness(&self.schema.resources, Vec::new())?;
        self.check_command_name_collisions(&self.schema.resources, Vec::new())?;
        self.check_enum_types_have_values(&self.schema.resources, Vec::new())?;
        Ok(())
    }

    /// Root verb names must be unique among themselves.
    fn check_root_verb_uniqueness(&self) -> anyhow::Result<()> {
        let mut seen = std::collections::HashSet::new();
        for op in &self.schema.root.operations {
            if !seen.insert(op.verb.as_str()) {
                anyhow::bail!("duplicate verb `{}` in [root]", op.verb);
            }
        }
        Ok(())
    }

    /// A root verb must not shadow a top-level resource name, because both
    /// live as direct variants of the generated `Commands` enum.
    fn check_root_vs_resource_collisions(&self) -> anyhow::Result<()> {
        let resource_names: std::collections::HashSet<&str> = self
            .schema
            .resources
            .iter()
            .map(|r| r.name.as_str())
            .collect();
        for op in &self.schema.root.operations {
            if resource_names.contains(op.verb.as_str()) {
                anyhow::bail!(
                    "[root] verb `{}` collides with a top-level resource of the same name",
                    op.verb
                );
            }
        }
        Ok(())
    }

    fn check_resource_name_uniqueness(
        &self,
        resources: &[ResourceSchema],
        parent_path: Option<&str>,
    ) -> anyhow::Result<()> {
        let mut seen = HashSet::new();
        for resource in resources {
            if !seen.insert(resource.name.as_str()) {
                match parent_path {
                    Some(parent) => {
                        anyhow::bail!(
                            "duplicate sub-resource name `{}` under resource `{}`",
                            resource.name,
                            parent
                        );
                    }
                    None => anyhow::bail!("duplicate resource name: `{}`", resource.name),
                }
            }

            let full_path = match parent_path {
                Some(parent) => format!("{parent}.{}", resource.name),
                None => resource.name.clone(),
            };
            self.check_resource_name_uniqueness(&resource.subresources, Some(&full_path))?;
        }
        Ok(())
    }

    fn check_verb_uniqueness(
        &self,
        resources: &[ResourceSchema],
        parent_path: Vec<String>,
    ) -> anyhow::Result<()> {
        for resource in resources {
            let mut path = parent_path.clone();
            path.push(resource.name.clone());

            let mut seen = HashSet::new();
            for op in &resource.operations {
                if !seen.insert(op.verb.as_str()) {
                    anyhow::bail!(
                        "duplicate verb `{}` in resource `{}`",
                        op.verb,
                        path.join(".")
                    );
                }
            }

            self.check_verb_uniqueness(&resource.subresources, path)?;
        }
        Ok(())
    }

    fn check_enum_types_have_values(
        &self,
        resources: &[ResourceSchema],
        parent_path: Vec<String>,
    ) -> anyhow::Result<()> {
        for resource in resources {
            let mut path = parent_path.clone();
            path.push(resource.name.clone());
            let resource_path = path.join(".");

            for op in &resource.operations {
                for flag in &op.flags {
                    if matches!(flag.arg_type, ArgType::Enum) && flag.values.is_none() {
                        anyhow::bail!(
                            "flag `{}` in `{} {}` has type `enum` but no `values` list",
                            flag.name,
                            resource_path,
                            op.verb
                        );
                    }
                }
            }

            self.check_enum_types_have_values(&resource.subresources, path)?;
        }
        Ok(())
    }

    fn check_command_name_collisions(
        &self,
        resources: &[ResourceSchema],
        parent_path: Vec<String>,
    ) -> anyhow::Result<()> {
        for resource in resources {
            let mut path = parent_path.clone();
            path.push(resource.name.clone());
            let resource_path = path.join(".");

            let operation_names: HashSet<&str> = resource
                .operations
                .iter()
                .map(|op| op.verb.as_str())
                .collect();
            for child in &resource.subresources {
                if operation_names.contains(child.name.as_str()) {
                    anyhow::bail!(
                        "resource `{}` cannot have both an operation and sub-resource named `{}`",
                        resource_path,
                        child.name
                    );
                }
            }

            self.check_command_name_collisions(&resource.subresources, path)?;
        }
        Ok(())
    }
}
