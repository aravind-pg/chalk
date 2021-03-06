use crate::fallible::Fallible;
use crate::{ExClause, SimplifiedAnswer};
use crate::hh::HhGoal;
use std::fmt::Debug;
use std::hash::Hash;

crate mod prelude;

pub trait Context
    : Sized + Clone + Debug + ContextOps<Self> + Aggregate<Self> + TruncateOps<Self> + ResolventOps<Self>
    {
    /// Represents an inference table.
    type InferenceTable: InferenceTable<Self>;

    /// Represents a set of hypotheses that are assumed to be true.
    type Environment: Environment<Self>;

    /// Goals correspond to things we can prove.
    type Goal: Goal<Self>;

    /// A goal that can be targeted by a program clause. The SLG
    /// solver treats these opaquely; in contrast, it understands
    /// "meta" goals like `G1 && G2` and so forth natively.
    type DomainGoal: DomainGoal<Self>;

    /// A map between universes. These are produced when
    /// u-canonicalizing something; they map canonical results back to
    /// the universes from the original.
    type UniverseMap: UniverseMap<Self>;

    /// Represents a goal along with an environment.
    type GoalInEnvironment: GoalInEnvironment<Self>;

    /// A canonicalized `GoalInEnvironment` -- that is, one where all
    /// free inference variables have been bound into the canonical
    /// binder. See [the rustc-guide] for more information.
    ///
    /// [the rustc-guide]: https://rust-lang-nursery.github.io/rustc-guide/traits-canonicalization.html
    type CanonicalGoalInEnvironment: CanonicalGoalInEnvironment<Self>;

    /// A u-canonicalized `GoalInEnvironment` -- this is one where the
    /// free universes are renumbered to consecutive integers starting
    /// from U1 (but preserving their relative order).
    type UCanonicalGoalInEnvironment: UCanonicalGoalInEnvironment<Self>;

    /// Represents a region constraint that will be propagated back
    /// (but not verified).
    type RegionConstraint: ConstraintInEnvironment<Self>;

    /// Represents a substitution from the "canonical variables" found
    /// in a canonical goal to specific values.
    type Substitution: Substitution<Self>;

    /// Part of an answer: represents a canonicalized substitution,
    /// combined with region constraints. See [the rustc-guide] for more information.
    ///
    /// [the rustc-guide]: https://rust-lang-nursery.github.io/rustc-guide/traits-canonicalization.html#canonicalizing-the-query-result
    type CanonicalConstrainedSubst: CanonicalConstrainedSubst<Self>;

    /// A "higher-order" goal, quantified over some types and/or
    /// lifetimes. When you have a quantification, like `forall<T> { G
    /// }` or `exists<T> { G }`, this represents the `<T> { G }` part.
    ///
    /// (In Lambda Prolog, this would be a "lambda predicate", like `T
    /// \ Goal`).
    type BindersGoal: BindersGoal<Self>;

    /// A term that can be quantified over and unified -- in current
    /// Chalk, either a type or lifetime.
    type Parameter: Parameter<Self>;

    /// A rule like `DomainGoal :- Goal`.
    ///
    /// `resolvent_clause` combines a program-clause and a concrete
    /// goal we are trying to solve to produce an ex-clause.
    type ProgramClause: ProgramClause<Self>;

    /// A final solution that is passed back to the user. This is
    /// completely opaque to the SLG solver; it is produced by
    /// `make_solution`.
    type Solution;
}

/// "Truncation" (called "abstraction" in the papers referenced below)
/// refers to the act of modifying a goal or answer that has become
/// too large in order to guarantee termination. The SLG solver
/// doesn't care about the precise truncation function, so long as
/// it's deterministic and so forth.
///
/// Citations:
///
/// - Terminating Evaluation of Logic Programs with Finite Three-Valued Models
///   - Riguzzi and Swift; ACM Transactions on Computational Logic 2013
/// - Radial Restraint
///   - Grosof and Swift; 2013
pub trait TruncateOps<C: Context> {
    /// If `subgoal` is too large, return a truncated variant (else
    /// return `None`).
    fn truncate_goal(
        &self,
        infer: &mut C::InferenceTable,
        subgoal: &C::GoalInEnvironment,
    ) -> Option<C::GoalInEnvironment>;

    /// If `subst` is too large, return a truncated variant (else
    /// return `None`).
    fn truncate_answer(
        &self,
        infer: &mut C::InferenceTable,
        subst: &C::Substitution,
    ) -> Option<C::Substitution>;
}

pub trait ContextOps<C: Context> {
    /// True if this is a coinductive goal -- e.g., proving an auto trait.
    fn is_coinductive(&self, goal: &C::UCanonicalGoalInEnvironment) -> bool;

    /// Returns the set of program clauses that might apply to
    /// `goal`. (This set can be over-approximated, naturally.)
    fn program_clauses(
        &self,
        environment: &C::Environment,
        goal: &C::DomainGoal,
    ) -> Vec<C::ProgramClause>;

    fn goal_in_environment(environment: &C::Environment, goal: C::Goal) -> C::GoalInEnvironment;
}

pub trait ResolventOps<C: Context> {
    fn resolvent_clause(
        &self,
        infer: &mut C::InferenceTable,
        environment: &C::Environment,
        goal: &C::DomainGoal,
        subst: &C::Substitution,
        clause: &C::ProgramClause,
    ) -> Fallible<ExClause<C>>;

    fn apply_answer_subst(
        &self,
        infer: &mut C::InferenceTable,
        ex_clause: ExClause<C>,
        selected_goal: &C::GoalInEnvironment,
        answer_table_goal: &C::CanonicalGoalInEnvironment,
        canonical_answer_subst: &C::CanonicalConstrainedSubst,
    ) -> Fallible<ExClause<C>>;
}

pub trait Aggregate<C: Context> {
    fn make_solution(
        &self,
        root_goal: &C::CanonicalGoalInEnvironment,
        simplified_answers: impl IntoIterator<Item = SimplifiedAnswer<C>>,
    ) -> Option<C::Solution>;
}

pub trait UCanonicalGoalInEnvironment<C: Context>: Debug + Clone + Eq + Hash {
    fn canonical(&self) -> &C::CanonicalGoalInEnvironment;
    fn is_trivial_substitution(&self, canonical_subst: &C::CanonicalConstrainedSubst) -> bool;
}

pub trait CanonicalGoalInEnvironment<C: Context>: Debug + Clone {
    fn substitute(&self, subst: &C::Substitution) -> (C::Environment, C::Goal);
}

pub trait GoalInEnvironment<C: Context>: Debug + Clone + Eq + Ord + Hash {
    fn environment(&self) -> &C::Environment;
}

pub trait Environment<C: Context>: Debug + Clone + Eq + Ord + Hash {
    // Used by: simplify
    fn add_clauses(&self, clauses: impl IntoIterator<Item = C::DomainGoal>) -> Self;
}

pub trait InferenceTable<C: Context>: Clone {
    type UnificationResult: UnificationResult<C>;

    fn new() -> Self;

    // Used by: simplify
    fn instantiate_binders_universally(&mut self, arg: &C::BindersGoal) -> C::Goal;

    // Used by: simplify
    fn instantiate_binders_existentially(&mut self, arg: &C::BindersGoal) -> C::Goal;

    // Used by: logic
    fn instantiate_universes<'v>(
        &mut self,
        value: &'v C::UCanonicalGoalInEnvironment,
    ) -> &'v C::CanonicalGoalInEnvironment;

    // Used by: logic (but for debugging only)
    fn debug_ex_clause(&mut self, value: &'v ExClause<C>) -> Box<dyn Debug + 'v>;

    // Used by: logic (but for debugging only)
    fn debug_goal(&mut self, goal: &'v C::GoalInEnvironment) -> Box<dyn Debug + 'v>;

    // Used by: logic
    fn canonicalize_goal(&mut self, value: &C::GoalInEnvironment) -> C::CanonicalGoalInEnvironment;

    // Used by: logic
    fn canonicalize_constrained_subst(
        &mut self,
        subst: C::Substitution,
        constraints: Vec<C::RegionConstraint>,
    ) -> C::CanonicalConstrainedSubst;

    // Used by: logic
    fn u_canonicalize_goal(
        &mut self,
        value: &C::CanonicalGoalInEnvironment,
    ) -> (C::UCanonicalGoalInEnvironment, C::UniverseMap);

    // Used by: logic
    fn fresh_subst_for_goal(&mut self, goal: &C::CanonicalGoalInEnvironment) -> C::Substitution;

    // Used by: logic
    fn invert_goal(&mut self, value: &C::GoalInEnvironment) -> Option<C::GoalInEnvironment>;

    // Used by: simplify
    fn unify_parameters(
        &mut self,
        environment: &C::Environment,
        a: &C::Parameter,
        b: &C::Parameter,
    ) -> Fallible<Self::UnificationResult>;
}

pub trait Substitution<C: Context>: Clone + Debug {}

pub trait CanonicalConstrainedSubst<C: Context>: Clone + Debug + Eq + Hash + Ord {
    fn empty_constraints(&self) -> bool;
}

pub trait ConstraintInEnvironment<C: Context>: Clone + Debug + Eq + Hash + Ord {}

pub trait DomainGoal<C: Context>: Clone + Debug + Eq + Hash + Ord {
    fn into_goal(self) -> C::Goal;
}

pub trait Goal<C: Context>: Clone + Debug + Eq + Hash + Ord {
    fn cannot_prove() -> Self;
    fn into_hh_goal(self) -> HhGoal<C>;
}

pub trait Parameter<C: Context>: Clone + Debug + Eq + Hash + Ord {}

pub trait ProgramClause<C: Context>: Debug {}

pub trait BindersGoal<C: Context>: Clone + Debug + Eq + Hash + Ord {}

pub trait UniverseMap<C: Context>: Clone + Debug {
    fn map_goal_from_canonical(
        &self,
        value: &C::CanonicalGoalInEnvironment,
    ) -> C::CanonicalGoalInEnvironment;

    fn map_subst_from_canonical(
        &self,
        value: &C::CanonicalConstrainedSubst,
    ) -> C::CanonicalConstrainedSubst;
}

pub trait UnificationResult<C: Context> {
    fn into_ex_clause(self, ex_clause: &mut ExClause<C>);
}
