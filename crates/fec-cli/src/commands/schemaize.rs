use crate::cli::SchemaizeArgs;

pub fn schemaize(args: SchemaizeArgs) -> anyhow::Result<()> {
    super::export::sqlite::cmd_schemaize(args.path)
}
