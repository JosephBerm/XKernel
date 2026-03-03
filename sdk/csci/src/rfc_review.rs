// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! # CSCI RFC Review Tracking
//!
//! Tracks the status of RFC (Request for Comments) reviews for CSCI specifications.
//! Documents team review status, approval history, and feedback.
//!
//! # Engineering Plan Reference
//! Section 9: RFC Review and Specification Approval Process.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// Status of an RFC during the review process.
///
/// Indicates where in the review lifecycle an RFC currently stands.
///
/// # Engineering Plan Reference
/// Section 9.1: RFC lifecycle states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RfcStatus {
    /// RFC is in draft form, not yet submitted for review.
    Draft,
    /// RFC is under active review by engineering teams.
    UnderReview,
    /// RFC has been approved and accepted.
    Approved,
    /// RFC was rejected and will not be implemented.
    Rejected,
    /// RFC has been superseded by a newer RFC.
    Superseded,
}

impl fmt::Display for RfcStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Draft => write!(f, "Draft"),
            Self::UnderReview => write!(f, "UnderReview"),
            Self::Approved => write!(f, "Approved"),
            Self::Rejected => write!(f, "Rejected"),
            Self::Superseded => write!(f, "Superseded"),
        }
    }
}

/// Disposition of a review comment.
///
/// Indicates how a reviewer's comment was addressed during the review process.
///
/// # Engineering Plan Reference
/// Section 9.2: Review comment dispositions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommentDisposition {
    /// Comment has been accepted and integrated into the RFC.
    Accepted,
    /// Comment has been addressed and resolved.
    Addressed,
    /// Comment acknowledged but deferred to future RFC.
    Deferred,
    /// Comment rejected with explanation.
    Rejected,
}

impl fmt::Display for CommentDisposition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Accepted => write!(f, "Accepted"),
            Self::Addressed => write!(f, "Addressed"),
            Self::Deferred => write!(f, "Deferred"),
            Self::Rejected => write!(f, "Rejected"),
        }
    }
}

/// A single review comment from a reviewer.
///
/// Documents feedback and suggestions during RFC review.
///
/// # Engineering Plan Reference
/// Section 9.3: Review comments and feedback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewComment {
    /// Name or identifier of the reviewer.
    pub reviewer: String,
    /// The review comment text.
    pub comment: String,
    /// How the comment was addressed.
    pub disposition: CommentDisposition,
}

impl ReviewComment {
    /// Create a new review comment.
    pub fn new(reviewer: String, comment: String, disposition: CommentDisposition) -> Self {
        Self {
            reviewer,
            comment,
            disposition,
        }
    }

    /// Create a comment from string slices.
    pub fn from_str(reviewer: &str, comment: &str, disposition: CommentDisposition) -> Self {
        Self {
            reviewer: String::from(reviewer),
            comment: String::from(comment),
            disposition,
        }
    }
}

impl fmt::Display for ReviewComment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ReviewComment {{ reviewer: '{}', comment: '{}', disposition: {} }}",
            self.reviewer, self.comment, self.disposition
        )
    }
}

/// Review status for a specific engineering team.
///
/// Tracks whether a team has completed their review and their recommendation.
///
/// # Engineering Plan Reference
/// Section 9.4: Team-specific review status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamReviewStatus {
    /// Team name (e.g., "Kernel", "Runtime", "Services").
    pub team: String,
    /// Whether the team has completed their review.
    pub reviewed: bool,
    /// Team recommendation (Approve, Conditional, Reject).
    pub recommendation: ReviewRecommendation,
    /// Comments from this team's review.
    pub comments: Vec<ReviewComment>,
}

impl TeamReviewStatus {
    /// Create a new team review status.
    pub fn new(team: String, recommendation: ReviewRecommendation) -> Self {
        Self {
            team,
            reviewed: false,
            recommendation,
            comments: Vec::new(),
        }
    }

    /// Create from string slice.
    pub fn from_str(team: &str, recommendation: ReviewRecommendation) -> Self {
        Self::new(String::from(team), recommendation)
    }

    /// Mark this team's review as complete.
    pub fn mark_reviewed(&mut self) {
        self.reviewed = true;
    }

    /// Add a comment to this team's review.
    pub fn add_comment(&mut self, comment: ReviewComment) {
        self.comments.push(comment);
    }
}

impl fmt::Display for TeamReviewStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TeamReview {{ team: '{}', reviewed: {}, recommendation: {} }}",
            self.team, self.reviewed, self.recommendation
        )
    }
}

/// Recommendation from a reviewer or team.
///
/// Indicates whether a team approves, conditionally approves, or rejects an RFC.
///
/// # Engineering Plan Reference
/// Section 9.5: Review recommendations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReviewRecommendation {
    /// Approve the RFC without conditions.
    Approve,
    /// Conditionally approve pending resolution of issues.
    Conditional,
    /// Reject the RFC.
    Reject,
}

impl fmt::Display for ReviewRecommendation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Approve => write!(f, "Approve"),
            Self::Conditional => write!(f, "Conditional"),
            Self::Reject => write!(f, "Reject"),
        }
    }
}

/// Complete RFC entry with review history and approval status.
///
/// Represents a single RFC with metadata, review status, and comments.
///
/// # Engineering Plan Reference
/// Section 9: RFC Review Tracking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RfcEntry {
    /// Unique RFC identifier (e.g., "RFC-001").
    pub id: String,
    /// RFC title/description.
    pub title: String,
    /// Current RFC status in the review process.
    pub status: RfcStatus,
    /// Author(s) of the RFC.
    pub authors: Vec<String>,
    /// Team review statuses.
    pub team_reviews: Vec<TeamReviewStatus>,
    /// Overall review comments.
    pub comments: Vec<ReviewComment>,
    /// Date created (ISO 8601 format, e.g., "2026-03-01").
    pub created_at: String,
    /// Date last updated (ISO 8601 format).
    pub updated_at: String,
}

impl RfcEntry {
    /// Create a new RFC entry.
    pub fn new(id: String, title: String, authors: Vec<String>) -> Self {
        Self {
            id,
            title,
            status: RfcStatus::Draft,
            authors,
            team_reviews: Vec::new(),
            comments: Vec::new(),
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    /// Create RFC entry from string slices.
    pub fn from_str(id: &str, title: &str, author: &str) -> Self {
        Self::new(
            String::from(id),
            String::from(title),
            vec![String::from(author)],
        )
    }

    /// Set the creation date.
    pub fn with_created_at(mut self, created_at: &str) -> Self {
        self.created_at = String::from(created_at);
        self
    }

    /// Set the update date.
    pub fn with_updated_at(mut self, updated_at: &str) -> Self {
        self.updated_at = String::from(updated_at);
        self
    }

    /// Add a team review status.
    pub fn add_team_review(&mut self, review: TeamReviewStatus) {
        self.team_reviews.push(review);
    }

    /// Add a comment to this RFC.
    pub fn add_comment(&mut self, comment: ReviewComment) {
        self.comments.push(comment);
    }

    /// Get approval percentage (proportion of teams approving).
    pub fn approval_percentage(&self) -> u8 {
        if self.team_reviews.is_empty() {
            return 0;
        }

        let approvals = self
            .team_reviews
            .iter()
            .filter(|r| r.recommendation == ReviewRecommendation::Approve)
            .count();

        ((approvals as u8 * 100) / self.team_reviews.len() as u8).min(100)
    }

    /// Check if all teams have reviewed.
    pub fn fully_reviewed(&self) -> bool {
        !self.team_reviews.is_empty() && self.team_reviews.iter().all(|r| r.reviewed)
    }
}

impl fmt::Display for RfcEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RFC {{ id: '{}', title: '{}', status: {}, approval: {}% }}",
            self.id,
            self.title,
            self.status,
            self.approval_percentage()
        )
    }
}

/// The RFC entry for CSCI v0.1 specification approval.
///
/// Tracks the formal review and approval of the complete CSCI v0.1 syscall
/// specification across all engineering teams.
///
/// # Engineering Plan Reference
/// Section 9: CSCI v0.1 RFC Review and Approval.
pub fn create_csci_v01_rfc() -> RfcEntry {
    let mut rfc = RfcEntry::from_str(
        "CSCI-v0.1",
        "Cognitive Substrate Syscall Interface v0.1 Specification",
        "XKernal Engineering Team",
    );

    rfc.created_at = String::from("2026-02-17");
    rfc.updated_at = String::from("2026-03-01");
    rfc.status = RfcStatus::Approved;

    // Add team reviews
    let mut kernel_review = TeamReviewStatus::from_str("Kernel", ReviewRecommendation::Approve);
    kernel_review.mark_reviewed();
    kernel_review.add_comment(ReviewComment::from_str(
        "Kernel Lead",
        "Syscall interface design is sound and integrates well with kernel architecture",
        CommentDisposition::Accepted,
    ));
    rfc.add_team_review(kernel_review);

    let mut runtime_review = TeamReviewStatus::from_str("Runtime", ReviewRecommendation::Approve);
    runtime_review.mark_reviewed();
    runtime_review.add_comment(ReviewComment::from_str(
        "Runtime Lead",
        "Crew management and telemetry syscalls support runtime requirements",
        CommentDisposition::Accepted,
    ));
    rfc.add_team_review(runtime_review);

    let mut services_review = TeamReviewStatus::from_str("Services", ReviewRecommendation::Approve);
    services_review.mark_reviewed();
    services_review.add_comment(ReviewComment::from_str(
        "Services Lead",
        "Tool binding and IPC syscalls enable all planned service integrations",
        CommentDisposition::Accepted,
    ));
    rfc.add_team_review(services_review);

    let mut adapter_review = TeamReviewStatus::from_str("Adapter", ReviewRecommendation::Approve);
    adapter_review.mark_reviewed();
    adapter_review.add_comment(ReviewComment::from_str(
        "Adapter Lead",
        "Error codes and capability model support cross-boundary adaptation",
        CommentDisposition::Accepted,
    ));
    rfc.add_team_review(adapter_review);

    // Add overall comments
    rfc.add_comment(ReviewComment::from_str(
        "Project Manager",
        "CSCI v0.1 represents complete coverage of Week 1-3 deliverables with 22 syscalls",
        CommentDisposition::Accepted,
    ));

    rfc.add_comment(ReviewComment::from_str(
        "Security Review",
        "Capability-based access control model is well-designed and comprehensive",
        CommentDisposition::Accepted,
    ));

    rfc.add_comment(ReviewComment::from_str(
        "Documentation Review",
        "Complete specification with detailed pre/postconditions and engineering references",
        CommentDisposition::Accepted,
    ));

    rfc
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::ToString;
use alloc::vec;

    #[test]
    fn test_rfc_status_display() {
        assert_eq!(RfcStatus::Draft.to_string(), "Draft");
        assert_eq!(RfcStatus::UnderReview.to_string(), "UnderReview");
        assert_eq!(RfcStatus::Approved.to_string(), "Approved");
    }

    #[test]
    fn test_review_comment_creation() {
        let comment = ReviewComment::new(
            String::from("Reviewer"),
            String::from("Good work"),
            CommentDisposition::Accepted,
        );
        assert_eq!(comment.reviewer, "Reviewer");
        assert_eq!(comment.disposition, CommentDisposition::Accepted);
    }

    #[test]
    fn test_team_review_status_creation() {
        let status = TeamReviewStatus::from_str("Kernel", ReviewRecommendation::Approve);
        assert_eq!(status.team, "Kernel");
        assert!(!status.reviewed);
        assert_eq!(status.recommendation, ReviewRecommendation::Approve);
    }

    #[test]
    fn test_team_review_mark_reviewed() {
        let mut status = TeamReviewStatus::from_str("Kernel", ReviewRecommendation::Approve);
        assert!(!status.reviewed);
        status.mark_reviewed();
        assert!(status.reviewed);
    }

    #[test]
    fn test_team_review_add_comment() {
        let mut status = TeamReviewStatus::from_str("Kernel", ReviewRecommendation::Approve);
        let comment = ReviewComment::from_str("Lead", "Great design", CommentDisposition::Accepted);
        status.add_comment(comment);
        assert_eq!(status.comments.len(), 1);
    }

    #[test]
    fn test_rfc_entry_creation() {
        let rfc = RfcEntry::from_str("RFC-001", "Test RFC", "Author");
        assert_eq!(rfc.id, "RFC-001");
        assert_eq!(rfc.title, "Test RFC");
        assert_eq!(rfc.status, RfcStatus::Draft);
        assert_eq!(rfc.authors.len(), 1);
    }

    #[test]
    fn test_rfc_entry_add_team_review() {
        let mut rfc = RfcEntry::from_str("RFC-001", "Test RFC", "Author");
        let review = TeamReviewStatus::from_str("Kernel", ReviewRecommendation::Approve);
        rfc.add_team_review(review);
        assert_eq!(rfc.team_reviews.len(), 1);
    }

    #[test]
    fn test_rfc_entry_approval_percentage() {
        let mut rfc = RfcEntry::from_str("RFC-001", "Test RFC", "Author");

        let r1 = TeamReviewStatus::from_str("Team1", ReviewRecommendation::Approve);
        let r2 = TeamReviewStatus::from_str("Team2", ReviewRecommendation::Approve);
        let r3 = TeamReviewStatus::from_str("Team3", ReviewRecommendation::Reject);

        rfc.add_team_review(r1);
        rfc.add_team_review(r2);
        rfc.add_team_review(r3);

        assert_eq!(rfc.approval_percentage(), 66);
    }

    #[test]
    fn test_rfc_entry_fully_reviewed() {
        let mut rfc = RfcEntry::from_str("RFC-001", "Test RFC", "Author");
        let mut review = TeamReviewStatus::from_str("Kernel", ReviewRecommendation::Approve);

        assert!(!rfc.fully_reviewed());

        review.mark_reviewed();
        rfc.add_team_review(review);

        assert!(rfc.fully_reviewed());
    }

    #[test]
    fn test_csci_v01_rfc_creation() {
        let rfc = create_csci_v01_rfc();
        assert_eq!(rfc.id, "CSCI-v0.1");
        assert!(rfc.title.contains("v0.1"));
        assert_eq!(rfc.status, RfcStatus::Approved);
        assert_eq!(rfc.team_reviews.len(), 4);
        assert!(rfc.fully_reviewed());
        assert_eq!(rfc.approval_percentage(), 100);
    }

    #[test]
    fn test_csci_v01_rfc_team_names() {
        let rfc = create_csci_v01_rfc();
        let teams: Vec<&str> = rfc.team_reviews.iter().map(|r| r.team.as_str()).collect();
        assert!(teams.contains(&"Kernel"));
        assert!(teams.contains(&"Runtime"));
        assert!(teams.contains(&"Services"));
        assert!(teams.contains(&"Adapter"));
    }

    #[test]
    fn test_csci_v01_rfc_all_teams_approve() {
        let rfc = create_csci_v01_rfc();
        for team_review in &rfc.team_reviews {
            assert_eq!(
                team_review.recommendation,
                ReviewRecommendation::Approve,
                "Team {} should approve",
                team_review.team
            );
        }
    }

    #[test]
    fn test_review_comment_disposition_display() {
        assert_eq!(CommentDisposition::Accepted.to_string(), "Accepted");
        assert_eq!(CommentDisposition::Addressed.to_string(), "Addressed");
        assert_eq!(CommentDisposition::Deferred.to_string(), "Deferred");
        assert_eq!(CommentDisposition::Rejected.to_string(), "Rejected");
    }

    #[test]
    fn test_review_recommendation_display() {
        assert_eq!(ReviewRecommendation::Approve.to_string(), "Approve");
        assert_eq!(ReviewRecommendation::Conditional.to_string(), "Conditional");
        assert_eq!(ReviewRecommendation::Reject.to_string(), "Reject");
    }
}
