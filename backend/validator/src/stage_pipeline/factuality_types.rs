use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchSnippet {
    pub title: Option<String>,
    pub snippet: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimVerdict {
    Supported,
    Contradicted,
    Unverifiable,
}

impl ClaimVerdict {
    pub fn as_str(self) -> &'static str {
        match self {
            ClaimVerdict::Supported => "supported",
            ClaimVerdict::Contradicted => "contradicted",
            ClaimVerdict::Unverifiable => "unverifiable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimVerification {
    pub claim: Claim,
    pub verdict: ClaimVerdict,
    pub evidence: Vec<SearchSnippet>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactualitySummary {
    pub supported: u32,
    pub contradicted: u32,
    pub unverifiable: u32,
    pub total: u32,
}

impl FactualitySummary {
    pub fn from_verifications(verifications: &[ClaimVerification]) -> Self {
        let mut supported = 0;
        let mut contradicted = 0;
        let mut unverifiable = 0;

        for verification in verifications {
            match verification.verdict {
                ClaimVerdict::Supported => supported += 1,
                ClaimVerdict::Contradicted => contradicted += 1,
                ClaimVerdict::Unverifiable => unverifiable += 1,
            }
        }

        Self {
            supported,
            contradicted,
            unverifiable,
            total: verifications.len() as u32,
        }
    }
}
