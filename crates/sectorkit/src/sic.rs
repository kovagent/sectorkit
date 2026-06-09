//! SEC SIC division taxonomy.
//!
//! The Standard Industrial Classification (SIC) was established in 1937 by
//! the US Department of Labor and is the legacy industry taxonomy that SEC
//! continues to publish per filer in 10-K cover-page metadata. SIC codes
//! are 4-digit identifiers grouped into 10 top-level **divisions** (A
//! through J), each spanning a contiguous SIC code range.
//!
//! The division boundaries below follow the SEC's published SIC list at
//! `https://www.sec.gov/info/edgar/siccodes.htm`.

use serde::{Deserialize, Serialize};
use std::fmt;

/// SEC SIC division -- the top-level open-source sector taxonomy.
///
/// Maps to the 10 divisions (A-J) published by the US Census Bureau and
/// maintained by SEC for filer metadata. Each SIC code falls into exactly
/// one division.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecSector {
    /// Division A -- SIC 0100-0999.
    AgricultureForestryFishing,
    /// Division B -- SIC 1000-1499.
    Mining,
    /// Division C -- SIC 1500-1799.
    Construction,
    /// Division D -- SIC 2000-3999.
    Manufacturing,
    /// Division E -- SIC 4000-4999.
    TransportationCommunicationsUtilities,
    /// Division F -- SIC 5000-5199.
    WholesaleTrade,
    /// Division G -- SIC 5200-5999.
    RetailTrade,
    /// Division H -- SIC 6000-6799.
    FinanceInsuranceRealEstate,
    /// Division I -- SIC 7000-8999.
    Services,
    /// Division J -- SIC 9100-9729.
    PublicAdministration,
    /// Unclassified -- SIC code outside any published division
    /// (e.g. SEC sentinel codes 9900-9999 for "non-classifiable" filers).
    Unclassified,
}

impl SecSector {
    /// Map a 4-digit SIC code to its SEC division.
    ///
    /// Returns [`SecSector::Unclassified`] for codes outside published
    /// division ranges (e.g. SEC sentinel codes such as 9995 "non-classifiable
    /// establishments").
    pub const fn from_sic(code: u32) -> Self {
        match code {
            100..=999 => Self::AgricultureForestryFishing,
            1000..=1499 => Self::Mining,
            1500..=1799 => Self::Construction,
            2000..=3999 => Self::Manufacturing,
            4000..=4999 => Self::TransportationCommunicationsUtilities,
            5000..=5199 => Self::WholesaleTrade,
            5200..=5999 => Self::RetailTrade,
            6000..=6799 => Self::FinanceInsuranceRealEstate,
            7000..=8999 => Self::Services,
            9100..=9729 => Self::PublicAdministration,
            _ => Self::Unclassified,
        }
    }

    /// Stable string slug used in Parquet/JSON snapshots.
    pub const fn as_slug(&self) -> &'static str {
        match self {
            Self::AgricultureForestryFishing => "agriculture_forestry_fishing",
            Self::Mining => "mining",
            Self::Construction => "construction",
            Self::Manufacturing => "manufacturing",
            Self::TransportationCommunicationsUtilities => {
                "transportation_communications_utilities"
            }
            Self::WholesaleTrade => "wholesale_trade",
            Self::RetailTrade => "retail_trade",
            Self::FinanceInsuranceRealEstate => "finance_insurance_real_estate",
            Self::Services => "services",
            Self::PublicAdministration => "public_administration",
            Self::Unclassified => "unclassified",
        }
    }

    /// Parse a slug back into a sector.
    pub fn from_slug(slug: &str) -> Option<Self> {
        Some(match slug {
            "agriculture_forestry_fishing" => Self::AgricultureForestryFishing,
            "mining" => Self::Mining,
            "construction" => Self::Construction,
            "manufacturing" => Self::Manufacturing,
            "transportation_communications_utilities" => {
                Self::TransportationCommunicationsUtilities
            }
            "wholesale_trade" => Self::WholesaleTrade,
            "retail_trade" => Self::RetailTrade,
            "finance_insurance_real_estate" => Self::FinanceInsuranceRealEstate,
            "services" => Self::Services,
            "public_administration" => Self::PublicAdministration,
            "unclassified" => Self::Unclassified,
            _ => return None,
        })
    }
}

impl fmt::Display for SecSector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_slug())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn division_boundaries() {
        assert_eq!(
            SecSector::from_sic(100),
            SecSector::AgricultureForestryFishing
        );
        assert_eq!(
            SecSector::from_sic(999),
            SecSector::AgricultureForestryFishing
        );
        assert_eq!(SecSector::from_sic(1000), SecSector::Mining);
        assert_eq!(SecSector::from_sic(1499), SecSector::Mining);
        assert_eq!(SecSector::from_sic(1500), SecSector::Construction);
        assert_eq!(SecSector::from_sic(2000), SecSector::Manufacturing);
        assert_eq!(SecSector::from_sic(3711), SecSector::Manufacturing);
        assert_eq!(
            SecSector::from_sic(4813),
            SecSector::TransportationCommunicationsUtilities
        );
        assert_eq!(SecSector::from_sic(5000), SecSector::WholesaleTrade);
        assert_eq!(SecSector::from_sic(5200), SecSector::RetailTrade);
        assert_eq!(
            SecSector::from_sic(6020),
            SecSector::FinanceInsuranceRealEstate
        );
        assert_eq!(SecSector::from_sic(7372), SecSector::Services);
        assert_eq!(SecSector::from_sic(9100), SecSector::PublicAdministration);
        assert_eq!(SecSector::from_sic(9995), SecSector::Unclassified);
    }

    #[test]
    fn slug_roundtrip() {
        for sector in [
            SecSector::AgricultureForestryFishing,
            SecSector::Mining,
            SecSector::Construction,
            SecSector::Manufacturing,
            SecSector::TransportationCommunicationsUtilities,
            SecSector::WholesaleTrade,
            SecSector::RetailTrade,
            SecSector::FinanceInsuranceRealEstate,
            SecSector::Services,
            SecSector::PublicAdministration,
            SecSector::Unclassified,
        ] {
            assert_eq!(SecSector::from_slug(sector.as_slug()), Some(sector));
        }
    }

    #[test]
    fn known_anchors() {
        // AAPL = 3571 (Electronic Computers) -> Manufacturing.
        assert_eq!(SecSector::from_sic(3571), SecSector::Manufacturing);
        // JPM = 6020 (State Commercial Banks) -> Finance.
        assert_eq!(
            SecSector::from_sic(6020),
            SecSector::FinanceInsuranceRealEstate
        );
        // GOOGL = 7372 (Prepackaged Software) -> Services.
        assert_eq!(SecSector::from_sic(7372), SecSector::Services);
    }
}
