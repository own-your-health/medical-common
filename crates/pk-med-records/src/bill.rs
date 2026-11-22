//! Bills from a provider's billing site (`DESIGN.md` §1.10): the code
//! systems this profile owns, the price components with their R4 types,
//! and the builders a producer writes an Invoice with, so no CLI spells a
//! code of its own and a store reads what every producer wrote.
//!
//! Amounts are `serde_json::Number`s the caller builds. A producer that keeps
//! the site's spelling (`125.00`) builds them with serde_json's
//! `arbitrary_precision`; this module never parses or reformats one.

use serde_json::{json, Map, Number, Value};

/// The code system of an Invoice's price components
/// (`lineItem[].priceComponent[].code`, `totalPriceComponent[].code`).
pub const COMPONENT_SYSTEM: &str = "urn:pk-med:bill-component";

/// The code system of a billing document's `DocumentReference.type`.
pub const DOC_TYPE_SYSTEM: &str = "urn:pk-med:doc-type";

/// The currency of every amount: the sites bill in US dollars.
pub const CURRENCY: &str = "USD";

/// One price component of a bill or of one of its lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Component {
    /// What the provider billed.
    Billed,
    /// What the insurer allows.
    Allowed,
    /// What the insurer paid.
    InsurancePayment,
    /// What the provider wrote off under the insurer's terms.
    InsuranceAdjustment,
    /// What the patient paid.
    PatientPayment,
    /// What the patient still owes.
    PatientResponsibility,
}

impl Component {
    /// Every component, in the order a bill lists them.
    pub const ALL: [Component; 6] = [
        Component::Billed,
        Component::Allowed,
        Component::InsurancePayment,
        Component::InsuranceAdjustment,
        Component::PatientPayment,
        Component::PatientResponsibility,
    ];

    /// The code in [`COMPONENT_SYSTEM`].
    pub fn code(self) -> &'static str {
        match self {
            Component::Billed => "billed",
            Component::Allowed => "allowed",
            Component::InsurancePayment => "insurance-payment",
            Component::InsuranceAdjustment => "insurance-adjustment",
            Component::PatientPayment => "patient-payment",
            Component::PatientResponsibility => "patient-responsibility",
        }
    }

    /// The R4 component type (`InvoicePriceComponentType`) it goes under.
    pub fn component_type(self) -> &'static str {
        match self {
            Component::Billed => "base",
            Component::Allowed | Component::PatientResponsibility => "informational",
            Component::InsurancePayment | Component::PatientPayment => "deduction",
            Component::InsuranceAdjustment => "discount",
        }
    }

    /// The coding's `display`.
    pub fn display(self) -> &'static str {
        match self {
            Component::Billed => "Billed charge",
            Component::Allowed => "Allowed amount",
            Component::InsurancePayment => "Insurance payment",
            Component::InsuranceAdjustment => "Insurance adjustment",
            Component::PatientPayment => "Patient payment",
            Component::PatientResponsibility => "Patient responsibility",
        }
    }

    /// The component a code in [`COMPONENT_SYSTEM`] names.
    pub fn from_code(code: &str) -> Option<Component> {
        Component::ALL.into_iter().find(|c| c.code() == code)
    }

    /// The price component, `amount` in [`CURRENCY`]:
    /// `{"type": …, "code": {"coding": [{"system", "code", "display"}]},
    /// "amount": {"value": amount, "currency": "USD"}}`.
    pub fn price_component(self, amount: Number) -> Value {
        json!({
            "type": self.component_type(),
            "code": {"coding": [{
                "system": COMPONENT_SYSTEM,
                "code": self.code(),
                "display": self.display(),
            }]},
            "amount": money(amount),
        })
    }

    /// Its amount in `components`, a `priceComponent` or
    /// `totalPriceComponent` array: the first entry under this component's
    /// type whose code is this one in [`COMPONENT_SYSTEM`]. `None` when there
    /// is none, or when its amount is not a number.
    pub fn amount_in(self, components: &Value) -> Option<&Number> {
        components.as_array()?.iter().find_map(|entry| {
            let named = entry["type"] == self.component_type()
                && entry["code"]["coding"].as_array().is_some_and(|codings| {
                    codings
                        .iter()
                        .any(|c| c["system"] == COMPONENT_SYSTEM && c["code"] == self.code())
                });
            match &entry["amount"]["value"] {
                Value::Number(n) if named => Some(n),
                _ => None,
            }
        })
    }
}

/// A billing document's kind, its `DocumentReference.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DocType {
    /// A statement.
    Bill,
    /// A receipt for a payment.
    Receipt,
}

impl DocType {
    /// The code in [`DOC_TYPE_SYSTEM`].
    pub fn code(self) -> &'static str {
        match self {
            DocType::Bill => "bill",
            DocType::Receipt => "receipt",
        }
    }

    /// The kind a code in [`DOC_TYPE_SYSTEM`] names.
    pub fn from_code(code: &str) -> Option<DocType> {
        [DocType::Bill, DocType::Receipt]
            .into_iter()
            .find(|d| d.code() == code)
    }

    /// `{"coding": [{"system": DOC_TYPE_SYSTEM, "code": …}], "text": …}`, the
    /// text being the code, which `documents/v1` shows as the document's
    /// name when its attachment has no title.
    pub fn concept(self) -> Value {
        json!({
            "coding": [{"system": DOC_TYPE_SYSTEM, "code": self.code()}],
            "text": self.code(),
        })
    }

    /// The kind `type` (a `DocumentReference.type`) names in
    /// [`DOC_TYPE_SYSTEM`].
    pub fn of(concept: &Value) -> Option<DocType> {
        concept["coding"].as_array()?.iter().find_map(|c| {
            (c["system"] == DOC_TYPE_SYSTEM)
                .then(|| c["code"].as_str().and_then(DocType::from_code))
                .flatten()
        })
    }
}

/// `{"value": amount, "currency": "USD"}`.
pub fn money(amount: Number) -> Value {
    json!({"value": amount, "currency": CURRENCY})
}

/// Sets an Invoice's `totalNet` and `totalGross` to `owed`, what the
/// patient still owes on the bill. R4's totals differ only by tax, which a
/// medical bill does not carry; the billed amount is
/// [`Component::Billed`], never a total.
pub fn set_totals(invoice: &mut Map<String, Value>, owed: Number) {
    invoice.insert("totalNet".into(), money(owed.clone()));
    invoice.insert("totalGross".into(), money(owed));
}

#[cfg(test)]
mod tests {
    use super::*;

    const INVOICE: &str = include_str!("../fixtures/invoice.json");

    fn number(text: &str) -> Number {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn each_component_has_the_code_and_r4_type_design_lists() {
        let table: Vec<(&str, &str)> = Component::ALL
            .iter()
            .map(|c| (c.component_type(), c.code()))
            .collect();
        assert_eq!(
            table,
            [
                ("base", "billed"),
                ("informational", "allowed"),
                ("deduction", "insurance-payment"),
                ("discount", "insurance-adjustment"),
                ("deduction", "patient-payment"),
                ("informational", "patient-responsibility"),
            ]
        );
        for c in Component::ALL {
            assert_eq!(Component::from_code(c.code()), Some(c));
        }
        assert_eq!(Component::from_code("tax"), None);
    }

    #[test]
    fn a_line_built_here_is_the_line_of_the_fixture() {
        let invoice: Value = serde_json::from_str(INVOICE).unwrap();
        let amounts = ["250.00", "180.00", "140.00", "70.00", "20.00", "20.00"];
        let built: Vec<Value> = Component::ALL
            .iter()
            .zip(amounts)
            .map(|(c, a)| c.price_component(number(a)))
            .collect();
        assert_eq!(Value::from(built), invoice["lineItem"][0]["priceComponent"]);
    }

    #[test]
    fn a_store_reads_billed_and_owed_from_the_totals() {
        let invoice: Value = serde_json::from_str(INVOICE).unwrap();
        let totals = &invoice["totalPriceComponent"];
        let billed = Component::Billed.amount_in(totals).unwrap();
        assert_eq!(billed.as_f64(), Some(250.0));
        let owes = Component::PatientResponsibility.amount_in(totals).unwrap();
        assert_eq!(Some(owes), invoice["totalNet"]["value"].as_number());
        assert_eq!(invoice["totalNet"], invoice["totalGross"]);
    }

    #[test]
    fn a_component_under_another_type_or_system_is_not_read() {
        let mut other_type = Component::Billed.price_component(number("1.00"));
        other_type["type"] = "informational".into();
        let mut other_system = Component::Billed.price_component(number("2.00"));
        other_system["code"]["coding"][0]["system"] = "urn:other".into();
        let mut no_number = Component::Billed.price_component(number("3.00"));
        no_number["amount"]["value"] = "3.00".into();
        let components = Value::from(vec![other_type, other_system, no_number]);
        assert_eq!(Component::Billed.amount_in(&components), None);
        assert_eq!(Component::Billed.amount_in(&Value::Null), None);
    }

    #[test]
    fn totals_are_both_what_the_patient_owes() {
        let mut invoice = Map::new();
        set_totals(&mut invoice, number("20.00"));
        let fixture: Value = serde_json::from_str(INVOICE).unwrap();
        assert_eq!(invoice["totalNet"], fixture["totalNet"]);
        assert_eq!(invoice["totalGross"], fixture["totalGross"]);
    }

    #[test]
    fn a_doc_type_reads_back_from_its_concept() {
        for d in [DocType::Bill, DocType::Receipt] {
            assert_eq!(DocType::of(&d.concept()), Some(d));
            assert_eq!(d.concept()["text"], d.code());
        }
        assert_eq!(
            DocType::of(
                &json!({"coding": [{"system": "urn:other", "code": "bill"}], "text": "bill"})
            ),
            None
        );
        assert_eq!(DocType::of(&json!({"text": "bill"})), None);
    }
}
