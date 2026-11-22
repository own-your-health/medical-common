# Fixtures

Synthetic data only, written by hand in the shapes DESIGN.md §1.10 gives a
bill. Dummy values follow `crates/pk-med-fhir/fixtures/README.md`: the
patient is `Robin Testperson`, identifiers start with `0000`, dates of care
fall in 2020, and a charge's text starts with `Synthetic `.

- `invoice.json`: one bill, one charge, every price component on its line
  and in its totals, and `totalNet` and `totalGross` what the patient still
  owes. Its amounts are spelled as a site prints them (`250.00`).
