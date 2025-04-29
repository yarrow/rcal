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
    Boolean(bool),
    CUType(CUType),
    Display(Display),
    Duration(SignedDuration),
    Encoding(Option<Base64>),
    FBType(FBType),
    Feature(Feature),
    FmtType(FmtType),
    Language(Language),
    Order(NonZeroUsize),
    ParamText(ParamText),
    PartStat(PartStat),
    Range(Option<ThisAndFuture>),
    RelType(RelType),
    Related(Related),
    Role(Role),
    ScheduleAgent(ScheduleAgent),
    ScheduleForceSend(ScheduleForceSend),
    ScheduleStatus(ScheduleStatus),
    SentBy(CalAddress),
    Size(u64),
    Text(String),
    Tzid(String),
    Uri(UriString),
    UriList(Vec<UriString>),
    Value(Value),
}

const ALTREP: usize = 0;
const CN: usize = 1;
const CUTYPE: usize = 2;
const DELEGATED_FROM: usize = 3;
const DELEGATED_TO: usize = 4;
const DERIVED: usize = 5;
const DIR: usize = 6;
const DISPLAY: usize = 7;
const EMAIL: usize = 8;
const ENCODING: usize = 9;
const FBTYPE: usize = 10;
const FEATURE: usize = 11;
const FILENAME: usize = 12;
const FMTTYPE: usize = 13;
const GAP: usize = 14;
const LABEL: usize = 15;
const LANGUAGE: usize = 16;
const LINKREL: usize = 17;
const MANAGED_ID: usize = 18;
const MEMBER: usize = 19;
const ORDER: usize = 20;
const PARTSTAT: usize = 21;
const RANGE: usize = 22;
const RELATED: usize = 23;
const RELTYPE: usize = 24;
const ROLE: usize = 25;
const RSVP: usize = 26;
const SCHEDULE_AGENT: usize = 27;
const SCHEDULE_FORCE_SEND: usize = 28;
const SCHEDULE_STATUS: usize = 29;
const SCHEMA: usize = 30;
const SENT_BY: usize = 31;
const SIZE: usize = 32;
const TZID: usize = 33;
const VALUE: usize = 34;
pub(crate) const NAMES: [&str; 35] = [
    "ALTREP",
    "CN",
    "CUTYPE",
    "DELEGATED-FROM",
    "DELEGATED-TO",
    "DERIVED",
    "DIR",
    "DISPLAY",
    "EMAIL",
    "ENCODING",
    "FBTYPE",
    "FEATURE",
    "FILENAME",
    "FMTTYPE",
    "GAP",
    "LABEL",
    "LANGUAGE",
    "LINKREL",
    "MANAGED-ID",
    "MEMBER",
    "ORDER",
    "PARTSTAT",
    "RANGE",
    "RELATED",
    "RELTYPE",
    "ROLE",
    "RSVP",
    "SCHEDULE-AGENT",
    "SCHEDULE-FORCE-SEND",
    "SCHEDULE-STATUS",
    "SCHEMA",
    "SENT-BY",
    "SIZE",
    "TZID",
    "VALUE",
];

#[allow(clippy::missing_panics_doc)] // We should only be `get`ing type that we `set`
impl Parameters {
    /// Get the `ALTREP` parameter ([RFC 5545, § 3.2.1](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.1)).
    #[must_use]
    pub fn altrep(&self) -> Option<&UriString> {
        match self.0.get(&ALTREP) {
            None => None,
            Some(ParameterValue::Uri(value)) => Some(value),
            _ => panic!("Unexpected type for {}", "ALTREP"),
        }
    }

    /// Set the `ALTREP` parameter ([RFC 5545, § 3.2.1](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.1)).
    pub fn set_altrep(&mut self, value: UriString) {
        self.0.insert(ALTREP, ParameterValue::Uri(value));
    }

    /// Get the `CN` parameter ([RFC 5545, § 3.2.2](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.2)).
    #[must_use]
    pub fn cn(&self) -> Option<&String> {
        match self.0.get(&CN) {
            None => None,
            Some(ParameterValue::Text(value)) => Some(value),
            _ => panic!("Unexpected type for {}", "CN"),
        }
    }

    /// Set the `CN` parameter ([RFC 5545, § 3.2.2](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.2)).
    pub fn set_cn(&mut self, value: String) {
        self.0.insert(CN, ParameterValue::Text(value));
    }

    /// Get the `CUTYPE` parameter ([RFC 5545, § 3.2.3](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.3)).
    #[must_use]
    pub fn cutype(&self) -> Option<&CUType> {
        match self.0.get(&CUTYPE) {
            None => None,
            Some(ParameterValue::CUType(value)) => Some(value),
            _ => panic!("Unexpected type for {}", "CUTYPE"),
        }
    }

    /// Set the `CUTYPE` parameter ([RFC 5545, § 3.2.3](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.3)).
    pub fn set_cutype(&mut self, value: CUType) {
        self.0.insert(CUTYPE, ParameterValue::CUType(value));
    }

    /// Get the `DELEGATED_FROM` parameter ([RFC 5545, § 3.2.4](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.4)).
    #[must_use]
    pub fn delegated_from(&self) -> Option<&Vec<UriString>> {
        match self.0.get(&DELEGATED_FROM) {
            None => None,
            Some(ParameterValue::UriList(value)) => Some(value),
            _ => panic!("Unexpected type for {}", "DELEGATED-FROM"),
        }
    }

    /// Set the `DELEGATED_FROM` parameter ([RFC 5545, § 3.2.4](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.4)).
    pub fn set_delegated_from(&mut self, value: Vec<UriString>) {
        self.0.insert(DELEGATED_FROM, ParameterValue::UriList(value));
    }

    /// Get the `DELEGATED_TO` parameter ([RFC 5545, § 3.2.5](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.5)).
    #[must_use]
    pub fn delegated_to(&self) -> Option<&Vec<UriString>> {
        match self.0.get(&DELEGATED_TO) {
            None => None,
            Some(ParameterValue::UriList(value)) => Some(value),
            _ => panic!("Unexpected type for {}", "DELEGATED-TO"),
        }
    }

    /// Set the `DELEGATED_TO` parameter ([RFC 5545, § 3.2.5](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.5)).
    pub fn set_delegated_to(&mut self, value: Vec<UriString>) {
        self.0.insert(DELEGATED_TO, ParameterValue::UriList(value));
    }

    /// Get the `DERIVED` parameter ([RFC 9073, § 5.3](https://datatracker.ietf.org/doc/html/rfc9073#section-5.3)).
    #[must_use]
    pub fn derived(&self) -> Option<bool> {
        match self.0.get(&DERIVED) {
            None => None,
            Some(ParameterValue::Boolean(value)) => Some(*value),
            _ => panic!("Unexpected type for {}", "DERIVED"),
        }
    }

    /// Set the `DERIVED` parameter ([RFC 9073, § 5.3](https://datatracker.ietf.org/doc/html/rfc9073#section-5.3)).
    pub fn set_derived(&mut self, value: bool) {
        self.0.insert(DERIVED, ParameterValue::Boolean(value));
    }

    /// Get the `DIR` parameter ([RFC 5545, § 3.2.6](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.6)).
    #[must_use]
    pub fn dir(&self) -> Option<&UriString> {
        match self.0.get(&DIR) {
            None => None,
            Some(ParameterValue::Uri(value)) => Some(value),
            _ => panic!("Unexpected type for {}", "DIR"),
        }
    }

    /// Set the `DIR` parameter ([RFC 5545, § 3.2.6](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.6)).
    pub fn set_dir(&mut self, value: UriString) {
        self.0.insert(DIR, ParameterValue::Uri(value));
    }

    /// Get the `DISPLAY` parameter ([RFC 7986, § 6.1](https://datatracker.ietf.org/doc/html/rfc7986#section-6.1)).
    #[must_use]
    pub fn display(&self) -> Option<&Display> {
        match self.0.get(&DISPLAY) {
            None => None,
            Some(ParameterValue::Display(value)) => Some(value),
            _ => panic!("Unexpected type for {}", "DISPLAY"),
        }
    }

    /// Set the `DISPLAY` parameter ([RFC 7986, § 6.1](https://datatracker.ietf.org/doc/html/rfc7986#section-6.1)).
    pub fn set_display(&mut self, value: Display) {
        self.0.insert(DISPLAY, ParameterValue::Display(value));
    }

    /// Get the `EMAIL` parameter ([RFC 7986, § 6.2](https://datatracker.ietf.org/doc/html/rfc7986#section-6.2)).
    #[must_use]
    pub fn email(&self) -> Option<&String> {
        match self.0.get(&EMAIL) {
            None => None,
            Some(ParameterValue::Text(value)) => Some(value),
            _ => panic!("Unexpected type for {}", "EMAIL"),
        }
    }

    /// Set the `EMAIL` parameter ([RFC 7986, § 6.2](https://datatracker.ietf.org/doc/html/rfc7986#section-6.2)).
    pub fn set_email(&mut self, value: String) {
        self.0.insert(EMAIL, ParameterValue::Text(value));
    }

    /// Get the `ENCODING` parameter ([RFC 5545, § 3.2.7](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.7)).
    /// RFC 5545 gives values of `8BIT` or `BASE64` but the effect of an `8BIT` value
    /// is the same as having no `ENCODING` parameterso we use the single-valued
    /// `Base64` type.
    #[must_use]
    pub fn encoding(&self) -> Option<Option<Base64>> {
        match self.0.get(&ENCODING) {
            None => None,
            Some(ParameterValue::Encoding(value)) => Some(*value),
            _ => panic!("Unexpected type for {}", "ENCODING"),
        }
    }

    /// Set the `ENCODING` parameter ([RFC 5545, § 3.2.7](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.7)).
    pub fn set_encoding(&mut self, value: Option<Base64>) {
        self.0.insert(ENCODING, ParameterValue::Encoding(value));
    }

    /// Get the `FBTYPE` parameter ([RFC 5545, § 3.2.9](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.9)).
    #[must_use]
    pub fn fbtype(&self) -> Option<&FBType> {
        match self.0.get(&FBTYPE) {
            None => None,
            Some(ParameterValue::FBType(value)) => Some(value),
            _ => panic!("Unexpected type for {}", "FBTYPE"),
        }
    }

    /// Set the `FBTYPE` parameter ([RFC 5545, § 3.2.9](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.9)).
    pub fn set_fbtype(&mut self, value: FBType) {
        self.0.insert(FBTYPE, ParameterValue::FBType(value));
    }

    /// Get the `FEATURE` parameter ([RFC 7986, § 6.3](https://datatracker.ietf.org/doc/html/rfc7986#section-6.3)).
    #[must_use]
    pub fn feature(&self) -> Option<&Feature> {
        match self.0.get(&FEATURE) {
            None => None,
            Some(ParameterValue::Feature(value)) => Some(value),
            _ => panic!("Unexpected type for {}", "FEATURE"),
        }
    }

    /// Set the `FEATURE` parameter ([RFC 7986, § 6.3](https://datatracker.ietf.org/doc/html/rfc7986#section-6.3)).
    pub fn set_feature(&mut self, value: Feature) {
        self.0.insert(FEATURE, ParameterValue::Feature(value));
    }

    /// Get the `FILENAME` parameter ([RFC 8607, § 4.2](https://datatracker.ietf.org/doc/html/rfc8607#section-4.2)).
    #[must_use]
    pub fn filename(&self) -> Option<&ParamText> {
        match self.0.get(&FILENAME) {
            None => None,
            Some(ParameterValue::ParamText(value)) => Some(value),
            _ => panic!("Unexpected type for {}", "FILENAME"),
        }
    }

    /// Set the `FILENAME` parameter ([RFC 8607, § 4.2](https://datatracker.ietf.org/doc/html/rfc8607#section-4.2)).
    pub fn set_filename(&mut self, value: ParamText) {
        self.0.insert(FILENAME, ParameterValue::ParamText(value));
    }

    /// Get the `FMTTYPE` parameter ([RFC 5545, § 3.2.8](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.8)).
    #[must_use]
    pub fn fmttype(&self) -> Option<&FmtType> {
        match self.0.get(&FMTTYPE) {
            None => None,
            Some(ParameterValue::FmtType(value)) => Some(value),
            _ => panic!("Unexpected type for {}", "FMTTYPE"),
        }
    }

    /// Set the `FMTTYPE` parameter ([RFC 5545, § 3.2.8](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.8)).
    pub fn set_fmttype(&mut self, value: FmtType) {
        self.0.insert(FMTTYPE, ParameterValue::FmtType(value));
    }

    /// Get the `GAP` parameter ([RFC 9253, § 6.2](https://datatracker.ietf.org/doc/html/rfc9253#section-6.2)).
    #[must_use]
    pub fn gap(&self) -> Option<SignedDuration> {
        match self.0.get(&GAP) {
            None => None,
            Some(ParameterValue::Duration(value)) => Some(*value),
            _ => panic!("Unexpected type for {}", "GAP"),
        }
    }

    /// Set the `GAP` parameter ([RFC 9253, § 6.2](https://datatracker.ietf.org/doc/html/rfc9253#section-6.2)).
    pub fn set_gap(&mut self, value: SignedDuration) {
        self.0.insert(GAP, ParameterValue::Duration(value));
    }

    /// Get the `LABEL` parameter ([RFC 7986, § 6.4](https://datatracker.ietf.org/doc/html/rfc7986#section-6.4)).
    #[must_use]
    pub fn label(&self) -> Option<&String> {
        match self.0.get(&LABEL) {
            None => None,
            Some(ParameterValue::Text(value)) => Some(value),
            _ => panic!("Unexpected type for {}", "LABEL"),
        }
    }

    /// Set the `LABEL` parameter ([RFC 7986, § 6.4](https://datatracker.ietf.org/doc/html/rfc7986#section-6.4)).
    pub fn set_label(&mut self, value: String) {
        self.0.insert(LABEL, ParameterValue::Text(value));
    }

    /// Get the `LANGUAGE` parameter ([RFC 5545, § 3.2.10](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.10)).
    #[must_use]
    pub fn language(&self) -> Option<&Language> {
        match self.0.get(&LANGUAGE) {
            None => None,
            Some(ParameterValue::Language(value)) => Some(value),
            _ => panic!("Unexpected type for {}", "LANGUAGE"),
        }
    }

    /// Set the `LANGUAGE` parameter ([RFC 5545, § 3.2.10](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.10)).
    pub fn set_language(&mut self, value: Language) {
        self.0.insert(LANGUAGE, ParameterValue::Language(value));
    }

    /// Get the `LINKREL` parameter ([RFC 9253, § 6.1](https://datatracker.ietf.org/doc/html/rfc9253#section-6.1)).
    #[must_use]
    pub fn linkrel(&self) -> Option<&UriString> {
        match self.0.get(&LINKREL) {
            None => None,
            Some(ParameterValue::Uri(value)) => Some(value),
            _ => panic!("Unexpected type for {}", "LINKREL"),
        }
    }

    /// Set the `LINKREL` parameter ([RFC 9253, § 6.1](https://datatracker.ietf.org/doc/html/rfc9253#section-6.1)).
    pub fn set_linkrel(&mut self, value: UriString) {
        self.0.insert(LINKREL, ParameterValue::Uri(value));
    }

    /// Get the `MANAGED_ID` parameter ([RFC 8607, § 4.3](https://datatracker.ietf.org/doc/html/rfc8607#section-4.3)).
    #[must_use]
    pub fn managed_id(&self) -> Option<&ParamText> {
        match self.0.get(&MANAGED_ID) {
            None => None,
            Some(ParameterValue::ParamText(value)) => Some(value),
            _ => panic!("Unexpected type for {}", "MANAGED-ID"),
        }
    }

    /// Set the `MANAGED_ID` parameter ([RFC 8607, § 4.3](https://datatracker.ietf.org/doc/html/rfc8607#section-4.3)).
    pub fn set_managed_id(&mut self, value: ParamText) {
        self.0.insert(MANAGED_ID, ParameterValue::ParamText(value));
    }

    /// Get the `MEMBER` parameter ([RFC 5545, § 3.2.11](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.11)).
    #[must_use]
    pub fn member(&self) -> Option<&Vec<UriString>> {
        match self.0.get(&MEMBER) {
            None => None,
            Some(ParameterValue::UriList(value)) => Some(value),
            _ => panic!("Unexpected type for {}", "MEMBER"),
        }
    }

    /// Set the `MEMBER` parameter ([RFC 5545, § 3.2.11](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.11)).
    pub fn set_member(&mut self, value: Vec<UriString>) {
        self.0.insert(MEMBER, ParameterValue::UriList(value));
    }

    /// Get the `ORDER` parameter ([RFC 9073, § 5.1](https://datatracker.ietf.org/doc/html/rfc9073#section-5.1)).
    #[must_use]
    pub fn order(&self) -> Option<NonZeroUsize> {
        match self.0.get(&ORDER) {
            None => None,
            Some(ParameterValue::Order(value)) => Some(*value),
            _ => panic!("Unexpected type for {}", "ORDER"),
        }
    }

    /// Set the `ORDER` parameter ([RFC 9073, § 5.1](https://datatracker.ietf.org/doc/html/rfc9073#section-5.1)).
    pub fn set_order(&mut self, value: NonZeroUsize) {
        self.0.insert(ORDER, ParameterValue::Order(value));
    }

    /// Get the `PARTSTAT` parameter ([RFC 5545, § 3.2.12](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.12)).
    #[must_use]
    pub fn partstat(&self) -> Option<&PartStat> {
        match self.0.get(&PARTSTAT) {
            None => None,
            Some(ParameterValue::PartStat(value)) => Some(value),
            _ => panic!("Unexpected type for {}", "PARTSTAT"),
        }
    }

    /// Set the `PARTSTAT` parameter ([RFC 5545, § 3.2.12](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.12)).
    pub fn set_partstat(&mut self, value: PartStat) {
        self.0.insert(PARTSTAT, ParameterValue::PartStat(value));
    }

    /// Get the `RANGE` parameter ([RFC 5545, § 3.2.13](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.13)).
    /// RFC 5545 says the only valid value for `RANGE` is `THISANDFUTURE`,
    /// so we have another single-valued type
    #[must_use]
    pub fn range(&self) -> Option<Option<ThisAndFuture>> {
        match self.0.get(&RANGE) {
            None => None,
            Some(ParameterValue::Range(value)) => Some(*value),
            _ => panic!("Unexpected type for {}", "RANGE"),
        }
    }

    /// Set the `RANGE` parameter ([RFC 5545, § 3.2.13](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.13)).
    pub fn set_range(&mut self, value: Option<ThisAndFuture>) {
        self.0.insert(RANGE, ParameterValue::Range(value));
    }

    /// Get the `RELATED` parameter ([RFC 5545, § 3.2.14](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.14)).
    #[must_use]
    pub fn related(&self) -> Option<Related> {
        match self.0.get(&RELATED) {
            None => None,
            Some(ParameterValue::Related(value)) => Some(*value),
            _ => panic!("Unexpected type for {}", "RELATED"),
        }
    }

    /// Set the `RELATED` parameter ([RFC 5545, § 3.2.14](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.14)).
    pub fn set_related(&mut self, value: Related) {
        self.0.insert(RELATED, ParameterValue::Related(value));
    }

    /// Get the `RELTYPE` parameter ([RFC 5545, § 3.2.15](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.15)).
    #[must_use]
    pub fn reltype(&self) -> Option<Related> {
        match self.0.get(&RELTYPE) {
            None => None,
            Some(ParameterValue::Related(value)) => Some(*value),
            _ => panic!("Unexpected type for {}", "RELTYPE"),
        }
    }

    /// Set the `RELTYPE` parameter ([RFC 5545, § 3.2.15](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.15)).
    pub fn set_reltype(&mut self, value: Related) {
        self.0.insert(RELTYPE, ParameterValue::Related(value));
    }

    /// Get the `ROLE` parameter ([RFC 5545, § 3.2.16](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.16)).
    #[must_use]
    pub fn role(&self) -> Option<&Role> {
        match self.0.get(&ROLE) {
            None => None,
            Some(ParameterValue::Role(value)) => Some(value),
            _ => panic!("Unexpected type for {}", "ROLE"),
        }
    }

    /// Set the `ROLE` parameter ([RFC 5545, § 3.2.16](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.16)).
    pub fn set_role(&mut self, value: Role) {
        self.0.insert(ROLE, ParameterValue::Role(value));
    }

    /// Get the `RSVP` parameter ([RFC 5545, § 3.2.17](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.17)).
    #[must_use]
    pub fn rsvp(&self) -> Option<bool> {
        match self.0.get(&RSVP) {
            None => None,
            Some(ParameterValue::Boolean(value)) => Some(*value),
            _ => panic!("Unexpected type for {}", "RSVP"),
        }
    }

    /// Set the `RSVP` parameter ([RFC 5545, § 3.2.17](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.17)).
    pub fn set_rsvp(&mut self, value: bool) {
        self.0.insert(RSVP, ParameterValue::Boolean(value));
    }

    /// Get the `SCHEDULE_AGENT` parameter ([RFC 6638, § 7.1](https://datatracker.ietf.org/doc/html/rfc6638#section-7.1)).
    #[must_use]
    pub fn schedule_agent(&self) -> Option<&ScheduleAgent> {
        match self.0.get(&SCHEDULE_AGENT) {
            None => None,
            Some(ParameterValue::ScheduleAgent(value)) => Some(value),
            _ => panic!("Unexpected type for {}", "SCHEDULE-AGENT"),
        }
    }

    /// Set the `SCHEDULE_AGENT` parameter ([RFC 6638, § 7.1](https://datatracker.ietf.org/doc/html/rfc6638#section-7.1)).
    pub fn set_schedule_agent(&mut self, value: ScheduleAgent) {
        self.0.insert(SCHEDULE_AGENT, ParameterValue::ScheduleAgent(value));
    }

    /// Get the `SCHEDULE_FORCE_SEND` parameter ([RFC 6638, § 7.2](https://datatracker.ietf.org/doc/html/rfc6638#section-7.2)).
    #[must_use]
    pub fn schedule_force_send(&self) -> Option<&ScheduleForceSend> {
        match self.0.get(&SCHEDULE_FORCE_SEND) {
            None => None,
            Some(ParameterValue::ScheduleForceSend(value)) => Some(value),
            _ => panic!("Unexpected type for {}", "SCHEDULE-FORCE-SEND"),
        }
    }

    /// Set the `SCHEDULE_FORCE_SEND` parameter ([RFC 6638, § 7.2](https://datatracker.ietf.org/doc/html/rfc6638#section-7.2)).
    pub fn set_schedule_force_send(&mut self, value: ScheduleForceSend) {
        self.0.insert(SCHEDULE_FORCE_SEND, ParameterValue::ScheduleForceSend(value));
    }

    /// Get the `SCHEDULE_STATUS` parameter ([RFC 6638, § 7.3](https://datatracker.ietf.org/doc/html/rfc6638#section-7.3)).
    #[must_use]
    pub fn schedule_status(&self) -> Option<&ScheduleStatus> {
        match self.0.get(&SCHEDULE_STATUS) {
            None => None,
            Some(ParameterValue::ScheduleStatus(value)) => Some(value),
            _ => panic!("Unexpected type for {}", "SCHEDULE-STATUS"),
        }
    }

    /// Set the `SCHEDULE_STATUS` parameter ([RFC 6638, § 7.3](https://datatracker.ietf.org/doc/html/rfc6638#section-7.3)).
    pub fn set_schedule_status(&mut self, value: ScheduleStatus) {
        self.0.insert(SCHEDULE_STATUS, ParameterValue::ScheduleStatus(value));
    }

    /// Get the `SCHEMA` parameter ([RFC 9073, § 5.2](https://datatracker.ietf.org/doc/html/rfc9073#section-5.2)).
    #[must_use]
    pub fn schema(&self) -> Option<&UriString> {
        match self.0.get(&SCHEMA) {
            None => None,
            Some(ParameterValue::Uri(value)) => Some(value),
            _ => panic!("Unexpected type for {}", "SCHEMA"),
        }
    }

    /// Set the `SCHEMA` parameter ([RFC 9073, § 5.2](https://datatracker.ietf.org/doc/html/rfc9073#section-5.2)).
    pub fn set_schema(&mut self, value: UriString) {
        self.0.insert(SCHEMA, ParameterValue::Uri(value));
    }

    /// Get the `SENT_BY` parameter ([RFC 5545, § 3.2.18](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.18)).
    #[must_use]
    pub fn sent_by(&self) -> Option<&CalAddress> {
        match self.0.get(&SENT_BY) {
            None => None,
            Some(ParameterValue::SentBy(value)) => Some(value),
            _ => panic!("Unexpected type for {}", "SENT-BY"),
        }
    }

    /// Set the `SENT_BY` parameter ([RFC 5545, § 3.2.18](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.18)).
    pub fn set_sent_by(&mut self, value: CalAddress) {
        self.0.insert(SENT_BY, ParameterValue::SentBy(value));
    }

    /// Get the `SIZE` parameter ([RFC 8607, § 4.1](https://datatracker.ietf.org/doc/html/rfc8607#section-4.1)).
    #[must_use]
    pub fn size(&self) -> Option<u64> {
        match self.0.get(&SIZE) {
            None => None,
            Some(ParameterValue::Size(value)) => Some(*value),
            _ => panic!("Unexpected type for {}", "SIZE"),
        }
    }

    /// Set the `SIZE` parameter ([RFC 8607, § 4.1](https://datatracker.ietf.org/doc/html/rfc8607#section-4.1)).
    pub fn set_size(&mut self, value: u64) {
        self.0.insert(SIZE, ParameterValue::Size(value));
    }

    /// Get the `TZID` parameter ([RFC 5545, § 3.2.19](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.19)).
    #[must_use]
    pub fn tzid(&self) -> Option<&String> {
        match self.0.get(&TZID) {
            None => None,
            Some(ParameterValue::Tzid(value)) => Some(value),
            _ => panic!("Unexpected type for {}", "TZID"),
        }
    }

    /// Set the `TZID` parameter ([RFC 5545, § 3.2.19](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.19)).
    pub fn set_tzid(&mut self, value: String) {
        self.0.insert(TZID, ParameterValue::Tzid(value));
    }

    /// Get the `VALUE` parameter ([RFC 5545, § 3.2.20](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.20)).
    #[must_use]
    pub fn value(&self) -> Option<&Value> {
        match self.0.get(&VALUE) {
            None => None,
            Some(ParameterValue::Value(value)) => Some(value),
            _ => panic!("Unexpected type for {}", "VALUE"),
        }
    }

    /// Set the `VALUE` parameter ([RFC 5545, § 3.2.20](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.20)).
    pub fn set_value(&mut self, value: Value) {
        self.0.insert(VALUE, ParameterValue::Value(value));
    }
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
