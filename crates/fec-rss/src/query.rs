pub const FEC_RSS_BASE_URL: &str = "https://efilingapps.fec.gov/rss/generate";

/// The feed's pre-defined filing type filters (`?preDefinedFilingType=`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Preset {
    #[default]
    All,
    Monthly,
    Quarterly,
    Presidential,
    Congressional,
    Pac,
}

impl Preset {
    /// The `preDefinedFilingType` value the FEC expects.
    pub fn code(self) -> &'static str {
        match self {
            Preset::All => "ALL",
            Preset::Monthly => "M",
            Preset::Quarterly => "Q",
            Preset::Presidential => "F3P",
            Preset::Congressional => "F3",
            Preset::Pac => "F3X",
        }
    }

    /// Human-readable label, e.g. for a TUI header.
    pub fn label(self) -> &'static str {
        match self {
            Preset::All => "All",
            Preset::Monthly => "Monthly",
            Preset::Quarterly => "Quarterly",
            Preset::Presidential => "Presidential (F3P)",
            Preset::Congressional => "Congressional (F3)",
            Preset::Pac => "PAC/Party (F3X)",
        }
    }
}

/// Filters for the feed URL. Any of `committees`/`forms`/`states`/`parties` switches
/// the feed to its custom-filter format, and `preset` is then ignored (the FEC
/// endpoint does not combine the two).
///
/// List values are passed through as-is, so comma-separate multiple values
/// (`"C00505412,C00513531"`). `forms`, `states` and `parties` are upper-cased.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FeedQuery {
    pub preset: Preset,
    pub committees: Option<String>,
    pub forms: Option<String>,
    pub states: Option<String>,
    pub parties: Option<String>,
}

impl FeedQuery {
    /// True when the custom-filter format is used (and `preset` is ignored).
    pub fn has_custom_filters(&self) -> bool {
        self.committees.is_some()
            || self.forms.is_some()
            || self.states.is_some()
            || self.parties.is_some()
    }

    /// The feed URL for these filters.
    pub fn url(&self) -> String {
        let mut url = String::from(FEC_RSS_BASE_URL);

        if self.has_custom_filters() {
            url.push('?');
            let mut params = Vec::new();
            if let Some(ref committees) = self.committees {
                params.push(format!("cids={committees}"));
            }
            if let Some(ref forms) = self.forms {
                params.push(format!("forms={}", forms.to_uppercase()));
            }
            if let Some(ref states) = self.states {
                params.push(format!("states={}", states.to_uppercase()));
            }
            if let Some(ref parties) = self.parties {
                params.push(format!("parties={}", parties.to_uppercase()));
            }
            url.push_str(&params.join("&"));
        } else {
            url.push_str("?preDefinedFilingType=");
            url.push_str(self.preset.code());
        }

        url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_url() {
        let q = FeedQuery {
            preset: Preset::Pac,
            ..Default::default()
        };
        assert_eq!(
            q.url(),
            "https://efilingapps.fec.gov/rss/generate?preDefinedFilingType=F3X"
        );
    }

    #[test]
    fn test_custom_url_ignores_preset() {
        let q = FeedQuery {
            preset: Preset::Monthly,
            committees: Some("C00505412,C00513531".into()),
            forms: Some("f3x".into()),
            states: Some("ca".into()),
            parties: None,
        };
        assert_eq!(
            q.url(),
            "https://efilingapps.fec.gov/rss/generate?cids=C00505412,C00513531&forms=F3X&states=CA"
        );
    }
}
