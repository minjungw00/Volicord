use crate::CandidateId;
use serde::{Deserialize, Serialize};
use volicord_context::{
    AgentRecommendation, ContextItemId, DecisionId, NonUserQuestionOutcome, Principal, ProjectId,
    QuestionAlternative, QuestionDependency, QuestionEstablishedFact, QuestionId,
    QuestionResearchState, SourceId, TimestampMicros,
};
use volicord_repository_intelligence::AnalysisSnapshotId;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CandidateKind {
    Observation,
    Hypothesis,
    SemanticClaim,
    QuestionCandidate,
    CheckpointCandidate,
    PromotionProposal,
    EngineeringChoiceDiscovery,
    MaterialityReview,
    LearningDeliberation,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CandidateCollectionMode {
    Automatic,
    ExplicitUserDirected,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CandidateCollectionScope {
    pub project_id: ProjectId,
    pub session: Option<String>,
    pub source_operation: Option<String>,
    pub candidate_kind: CandidateKind,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CollectionOptOutScope {
    pub project_id: ProjectId,
    pub session: Option<String>,
    pub source_operation: Option<String>,
    pub candidate_kind: Option<CandidateKind>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CollectionOptOut {
    pub scope: CollectionOptOutScope,
    pub opted_out: bool,
    pub effective_at: TimestampMicros,
    pub basis: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CandidateOrigin {
    pub actor: Principal,
    pub subsystem: String,
    pub session: Option<String>,
    pub provenance_summary: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct CandidateObservationBasis {
    pub source_basis: Vec<SourceId>,
    pub repository_snapshot: Option<String>,
    pub analysis_snapshot: Option<String>,
    pub execution: Option<String>,
    pub host_turn: Option<String>,
    pub other: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CandidateRetention {
    pub retained_until: Option<TimestampMicros>,
    pub basis: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CandidateCleanupKind {
    ExplicitDeletion,
    RetentionExpiry,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CandidateCleanup {
    pub kind: CandidateCleanupKind,
    pub basis: String,
    pub cleaned_at: TimestampMicros,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CandidateDisposition {
    PendingOrRetained,
    Promoted {
        canonical_question_id: QuestionId,
        promoted_at: TimestampMicros,
    },
    Dismissed {
        reason: String,
        dismissed_at: TimestampMicros,
    },
    ExpiredOrRetentionCleaned,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CandidateFreshness {
    Current,
    Stale,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum MaterialityStatus {
    Unassessed,
    NeedsEvidence,
    Material,
    NotMaterial,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MaterialityAssessment {
    pub status: MaterialityStatus,
    pub rationale: Option<String>,
    pub source_basis: Vec<SourceId>,
    pub assessed_by: Option<Principal>,
    pub assessed_at: Option<TimestampMicros>,
}

impl Default for MaterialityAssessment {
    fn default() -> Self {
        Self {
            status: MaterialityStatus::Unassessed,
            rationale: None,
            source_basis: Vec::new(),
            assessed_by: None,
            assessed_at: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DuplicateAssessment {
    Unassessed,
    NoDuplicate {
        basis: String,
    },
    DuplicateOf {
        question_id: QuestionId,
        basis: String,
    },
    SupersededBy {
        question_id: QuestionId,
        basis: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RepositoryResearchBasis {
    pub repository_snapshot: String,
    pub analysis_snapshot: Option<String>,
    pub capability: String,
    pub coverage: String,
    pub freshness: CandidateFreshness,
    pub source_basis: Vec<SourceId>,
    pub sufficient: bool,
    pub limits: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct QuestionCandidate {
    pub prompt_basis: String,
    pub known_facts: Vec<QuestionEstablishedFact>,
    pub assumptions: Vec<String>,
    pub uncertainty: Vec<String>,
    pub affected_scope: Vec<String>,
    pub possible_prerequisites: Vec<QuestionDependency>,
    pub source_basis: Vec<SourceId>,
    pub repository_basis: Vec<RepositoryResearchBasis>,
    pub freshness: CandidateFreshness,
    pub duplicate_assessment: DuplicateAssessment,
    pub materiality: MaterialityAssessment,
    pub presentation_order: Option<u64>,
    pub why_it_matters_now: String,
    pub alternatives: Vec<QuestionAlternative>,
    pub recommendation: AgentRecommendation,
    pub trade_offs: Vec<String>,
    pub known_limits: Vec<String>,
    pub what_the_answer_unlocks: Vec<String>,
    pub allowed_non_choice_dispositions: Vec<NonUserQuestionOutcome>,
    pub research_state: QuestionResearchState,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CandidateContent {
    pub bounded_summary: String,
    pub question: Option<QuestionCandidate>,
    pub engineering_choice_discovery: Option<EngineeringChoiceDiscovery>,
    pub materiality_review: Option<MaterialityReview>,
    pub learning_deliberation: Option<LearningDeliberation>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum EngineeringEffectCategory {
    PublicApiShapeOrSemantics,
    Compatibility,
    FailureOrErrorSemantics,
    PersistenceOrLifetime,
    PrivacyOrDisclosure,
    Security,
    UserVisibleBehaviorOrDefault,
    PerformanceOrResourceBehavior,
    ConcurrencyOrOperability,
    MaintenanceOrSupport,
    ImplementationInternal,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum EngineeringChoiceEvidenceState {
    Sufficient,
    ResearchRequired,
    PrototypeRequired,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum EngineeringChoiceRelationship {
    Independent,
    Coupled {
        choice_ids: Vec<String>,
        rationale: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EngineeringAlternative {
    pub alternative_id: String,
    pub summary: String,
    pub technical_consequences: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EngineeringChoice {
    pub choice_id: String,
    pub summary: String,
    pub affected_scope: Vec<String>,
    pub alternatives: Vec<EngineeringAlternative>,
    pub technical_consequences: Vec<String>,
    pub source_basis: Vec<SourceId>,
    pub effect_categories: Vec<EngineeringEffectCategory>,
    pub relationship: EngineeringChoiceRelationship,
    pub evidence_state: EngineeringChoiceEvidenceState,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EngineeringChoiceDiscovery {
    pub goal_context_id: ContextItemId,
    pub baseline_analysis_snapshot_id: AnalysisSnapshotId,
    pub choices: Vec<EngineeringChoice>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum MaterialOutcomeSignal {
    PublicApiSemantics,
    CliCompatibilityOrExitBehavior,
    ObservableFailurePolicy,
    PrivacyOrExternalDisclosure,
    SecurityPosture,
    UserVisibleDefault,
    MaintenanceOrSupportPolicy,
    OtherMaterialOutcome,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum WorkAuthorityBasisKind {
    RepositoryOrEnvironmentFact,
    AcceptedContract,
    ApplicableDecision,
    ExplicitDelegation,
    ResearchEvidence,
    PrototypeEvidence,
    DeferOrRevisitBasis,
    AgentRecommendation,
    LibraryOrConvention,
    ImplementationPreference,
    NoSettlingAuthority,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExplicitDelegationEvidence {
    pub goal_context_id: ContextItemId,
    pub user_turn_source_id: SourceId,
    pub verbatim_statement: String,
    pub affected_scope: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkAuthorityBasis {
    pub kinds: Vec<WorkAuthorityBasisKind>,
    pub summary: String,
    pub source_basis: Vec<SourceId>,
    pub contract_basis: Vec<String>,
    pub decision_basis: Vec<DecisionId>,
    pub research_basis: Vec<String>,
    pub explicit_delegation: Option<ExplicitDelegationEvidence>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ExploratoryDisposition {
    ResearchRequired,
    PrototypeRequired,
    DeferredWithRevisit,
    ResolvedByResearch,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum MaterialityDisposition {
    RepositoryOrEnvironmentFact,
    SettledAuthority,
    AgentOwnedImplementationChoice,
    DelegatedImplementationChoice,
    ExploratoryUncertainty {
        disposition: ExploratoryDisposition,
    },
    UnresolvedUserOwnedOutcome {
        resolution_decision_id: Option<DecisionId>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum LearningParticipation {
    Inactive,
    Active {
        user_turn_source_id: SourceId,
        verbatim_statement: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum LearningValueAssessment {
    Routine {
        rationale: String,
    },
    DeliberationWorthy {
        rationale: String,
        consequence_significance: Vec<String>,
        transferable_principles: Vec<String>,
        non_obvious_trade_offs: Vec<String>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MaterialityDimension {
    pub dimension_id: String,
    pub discovered_choice_ids: Vec<String>,
    pub summary: String,
    pub affected_scope: Vec<String>,
    pub material_consequences: Vec<String>,
    pub observable_signals: Vec<MaterialOutcomeSignal>,
    pub disposition: MaterialityDisposition,
    pub basis: WorkAuthorityBasis,
    pub learning_value: LearningValueAssessment,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MaterialityReview {
    pub goal_context_id: ContextItemId,
    pub baseline_analysis_snapshot_id: AnalysisSnapshotId,
    pub engineering_choice_discovery_candidate_id: CandidateId,
    pub first_review_analysis_snapshot_id: AnalysisSnapshotId,
    pub current_review_analysis_snapshot_id: AnalysisSnapshotId,
    pub first_review_preceded_meaningful_mutation: bool,
    pub rationale: String,
    pub learning_participation: LearningParticipation,
    pub dimensions: Vec<MaterialityDimension>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MaterialityReviewRevision {
    pub rationale: String,
    pub learning_participation: LearningParticipation,
    pub dimensions: Vec<MaterialityDimension>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LearningAlternativeSelection {
    pub choice_id: String,
    pub alternative_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum LearningInitialResponse {
    Select {
        selections: Vec<LearningAlternativeSelection>,
    },
    DelegateToAgent,
    Skip,
    RequestResearchOrPrototype {
        evidence_state: EngineeringChoiceEvidenceState,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LearningRecommendation {
    pub selections: Vec<LearningAlternativeSelection>,
    pub rationale: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LearningDeliberationRound {
    pub initial_response_source_id: SourceId,
    pub response: LearningInitialResponse,
    pub user_rationale: Option<String>,
    pub agent_feedback: Option<String>,
    pub agent_recommendation: Option<LearningRecommendation>,
    pub reconsideration_source_id: Option<SourceId>,
    pub reconsideration_rationale: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum LearningDeliberationState {
    AwaitingInitialResponse,
    AwaitingAgentFeedback {
        round: u32,
    },
    FeedbackProvided {
        round: u32,
    },
    Completed {
        round: u32,
        selected_alternatives: Vec<LearningAlternativeSelection>,
    },
    Delegated {
        round: u32,
    },
    Skipped {
        round: u32,
    },
    ResearchOrPrototypeRequired {
        round: u32,
        evidence_state: EngineeringChoiceEvidenceState,
    },
    ReconsiderationRequested {
        round: u32,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LearningDeliberation {
    pub goal_context_id: ContextItemId,
    pub baseline_analysis_snapshot_id: AnalysisSnapshotId,
    pub engineering_choice_discovery_candidate_id: CandidateId,
    pub materiality_review_candidate_id: CandidateId,
    pub dimension_id: String,
    pub discovered_choice_ids: Vec<String>,
    pub affected_scope: Vec<String>,
    pub problem: String,
    pub established_facts: Vec<String>,
    pub choices: Vec<EngineeringChoice>,
    pub rounds: Vec<LearningDeliberationRound>,
    pub state: LearningDeliberationState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateDraft {
    pub project_id: ProjectId,
    pub kind: CandidateKind,
    pub collection_mode: CandidateCollectionMode,
    pub origin: CandidateOrigin,
    pub collection_scope: CandidateCollectionScope,
    pub observation_basis: CandidateObservationBasis,
    pub observed_at: TimestampMicros,
    pub retention: CandidateRetention,
    pub content: CandidateContent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CandidateRecord {
    pub id: CandidateId,
    pub project_id: ProjectId,
    pub revision: u64,
    pub kind: CandidateKind,
    pub collection_mode: CandidateCollectionMode,
    pub origin: CandidateOrigin,
    pub collection_scope: CandidateCollectionScope,
    pub observation_basis: CandidateObservationBasis,
    pub created_at: TimestampMicros,
    pub observed_at: TimestampMicros,
    pub retention: CandidateRetention,
    pub disposition: CandidateDisposition,
    /// Candidate-local content cleanup is independent from promotion or
    /// dismissal. Once present, no lifecycle retry may recreate content.
    pub cleanup: Option<CandidateCleanup>,
    /// The canonical identity established by promotion. This survives later
    /// Candidate-content cleanup so local retention policy cannot erase the
    /// cross-store reconciliation basis.
    pub promotion_target: Option<QuestionId>,
    pub opt_out_state_at_collection: Vec<CollectionOptOut>,
    pub content: Option<CandidateContent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateReadBasis {
    pub project_id: ProjectId,
    pub candidates: Vec<CandidateRecord>,
    pub collection_policies: Vec<CollectionOptOut>,
    /// Related content is withheld while canonical forgetting cleanup is
    /// incomplete. This is a read barrier, not a lifecycle transition.
    pub withheld_for_canonical_forgetting: Vec<CandidateId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SubmissionOutcome {
    Stored(Box<CandidateRecord>),
    CollectionDisabled {
        matching_scopes: Vec<CollectionOptOut>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionResult {
    pub candidate_id: CandidateId,
    pub question_id: QuestionId,
    pub canonical_replayed: bool,
    pub candidate_reconciled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InquiryScope {
    pub project_id: ProjectId,
    pub material_scope: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuestionPresentation {
    pub question_id: QuestionId,
    pub displayed_revision: u64,
    pub prompt_basis: String,
    pub why_it_matters_now: String,
    pub material_scope: Vec<String>,
    pub established_facts: Vec<QuestionEstablishedFact>,
    pub alternatives: Vec<QuestionAlternative>,
    pub recommendation: AgentRecommendation,
    pub trade_offs: Vec<String>,
    pub uncertainty: Vec<String>,
    pub known_limits: Vec<String>,
    pub prerequisites: Vec<QuestionDependency>,
    pub what_the_answer_unlocks: Vec<String>,
    pub allowed_non_choice_dispositions: Vec<NonUserQuestionOutcome>,
}
