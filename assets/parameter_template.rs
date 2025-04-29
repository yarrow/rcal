use litemap::LiteMap;
use std::num::NonZeroUsize;

#[allow(clippy::wildcard_imports)]
use super::values::*;

///FIXME — add docs!
#[derive(Clone, Debug, Default)]
pub struct Parameters(LiteMap<usize, ParameterValue>);

///FIXME — add docs!
#[derive(Clone, Debug)]
pub enum ParameterValue {
    // ParameterValue
}

// const

#[allow(clippy::missing_panics_doc)] // We should only be `get`ing type that we `set`
impl Parameters {
    // Parameters
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;

    #[rustfmt::skip]
    const PARAMETER_IDS: [usize; 35] = [
        ALTREP, CN, CUTYPE, DELEGATED_FROM, DELEGATED_TO, DERIVED, DIR, DISPLAY,
        EMAIL, ENCODING, FBTYPE, FEATURE, FILENAME, FMTTYPE, GAP, LABEL,
        LANGUAGE, LINKREL, MANAGED_ID, MEMBER, ORDER, PARTSTAT, RANGE, RELATED,
        RELTYPE, ROLE, RSVP, SCHEDULE_AGENT, SCHEDULE_FORCE_SEND, SCHEDULE_STATUS,
        SCHEMA, SENT_BY, SIZE, TZID, VALUE
        ];
    #[rustfmt::skip]
    const PARAMETER_NAMES: [&str; 35] = [
        "ALTREP", "CN", "CUTYPE", "DELEGATED-FROM", "DELEGATED-TO", "DERIVED", "DIR", "DISPLAY",
        "EMAIL", "ENCODING", "FBTYPE", "FEATURE", "FILENAME", "FMTTYPE", "GAP", "LABEL",
        "LANGUAGE", "LINKREL", "MANAGED-ID", "MEMBER", "ORDER", "PARTSTAT", "RANGE", "RELATED",
        "RELTYPE", "ROLE", "RSVP", "SCHEDULE-AGENT", "SCHEDULE-FORCE-SEND", "SCHEDULE-STATUS",
        "SCHEMA", "SENT-BY", "SIZE", "TZID", "VALUE"
        ];

    #[test]
    fn parameter_ids_remain_in_order() {
        let expected: Vec<_> = (ALTREP..=VALUE).collect();
        assert_eq!(Vec::from(PARAMETER_IDS), expected);
    }
    #[test]
    fn calculated_names_and_listed_names_agree() {
        assert_eq!(NAMES, PARAMETER_NAMES);
    }
    #[test]
    fn parameter_names_are_sorted() {
        let mut sorted = PARAMETER_NAMES;
        sorted.sort_unstable();
        assert_eq!(NAMES, sorted);
    }
    #[test]
    fn parameter_names_correspond_to_parameter_ids() {
        use crate::names::{Lookup, ParameterId};
        let lookup = Lookup::new();
        let names_from_ids: Vec<_> = PARAMETER_IDS
            .into_iter()
            .map(|id| lookup.parameter_name(ParameterId(id)).unwrap().to_string())
            .collect();
        assert_eq!(names_from_ids, Vec::from(PARAMETER_NAMES));
    }
}
