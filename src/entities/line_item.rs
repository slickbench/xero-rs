use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Line amount types for tax calculations
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum LineAmountType {
    #[serde(alias = "EXCLUSIVE")]
    Exclusive,
    #[serde(alias = "INCLUSIVE")]
    Inclusive,
    #[serde(alias = "NOTAX")]
    NoTax,
}

/// Represents a line item in an invoice, quote, or other financial document.
///
/// # Discount Fields
///
/// Line items support two types of discounts:
/// - `discount_rate`: A percentage discount (e.g., 10.00 for 10%)
/// - `discount_amount`: A fixed amount discount (e.g., 25.00 for $25.00)
///
/// Note: `discount_amount` is only supported on ACCREC invoices and quotes.
/// ACCPAY invoices and credit notes in Xero do not support discounts.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LineItem {
    #[serde(rename = "LineItemID")]
    pub id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit_amount: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_amount: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_amount: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discount_rate: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discount_amount: Option<Decimal>,
    #[serde(default)]
    pub tracking: Vec<serde_json::Value>,
    #[serde(default)]
    pub validation_errors: Vec<serde_json::Value>,
}

impl LineItem {
    #[must_use]
    pub fn into_builder(self) -> Builder {
        let mut builder = Builder::new(self.description, self.quantity, self.unit_amount);
        builder.item_code = self.item_code;
        builder.account_code = self.account_code;
        builder.tax_type = self.tax_type;
        builder.discount_rate = self.discount_rate;
        builder.discount_amount = self.discount_amount;
        builder.id = Some(self.id);

        builder
    }
}

#[derive(Default, Debug, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Builder {
    #[serde(rename = "LineItemID")]
    pub id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_amount: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_rate: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_amount: Option<Decimal>,
}

impl Builder {
    #[must_use]
    pub fn new(
        description: Option<String>,
        quantity: Option<Decimal>,
        unit_amount: Option<Decimal>,
    ) -> Self {
        Builder {
            description,
            quantity,
            unit_amount,
            ..Self::default()
        }
    }

    /// Set the percentage discount for this line item
    #[must_use]
    pub fn with_discount_rate(mut self, rate: Decimal) -> Self {
        self.discount_rate = Some(rate);
        self
    }

    /// Set the fixed discount amount for this line item
    ///
    /// Note: Only supported on ACCREC invoices and quotes
    #[must_use]
    pub fn with_discount_amount(mut self, amount: Decimal) -> Self {
        self.discount_amount = Some(amount);
        self
    }

    /// Set the item code
    #[must_use]
    pub fn with_item_code(mut self, code: impl Into<String>) -> Self {
        self.item_code = Some(code.into());
        self
    }

    /// Set the account code
    #[must_use]
    pub fn with_account_code(mut self, code: impl Into<String>) -> Self {
        self.account_code = Some(code.into());
        self
    }

    /// Set the tax type
    #[must_use]
    pub fn with_tax_type(mut self, tax_type: impl Into<String>) -> Self {
        self.tax_type = Some(tax_type.into());
        self
    }
}
