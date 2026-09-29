use std::collections::{BTreeMap, BTreeSet};

use super::{
    model::{
        DomainChangeEnvelope, DomainChangeKind, DomainChangeSource, DomainInvalidationTarget,
        DomainProjectionWatermark, SelectiveRefreshNotice, SelectiveRefreshTarget,
    },
    store::{DomainChangeBatch, DomainChangeStoreError},
};

const MAX_NOTICE_VALUES: usize = 64;

/// One deterministic worker delivery unit. `through_sequence` is an internal
/// durable worker boundary, never a client-visible resume identifier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CoalescedSelectiveRefresh {
    through_sequence: u64,
    notices: Vec<SelectiveRefreshNotice>,
}

impl CoalescedSelectiveRefresh {
    pub(crate) fn through_sequence(&self) -> u64 {
        self.through_sequence
    }

    pub(crate) fn notices(&self) -> &[SelectiveRefreshNotice] {
        &self.notices
    }
}

pub(crate) fn coalesce_domain_changes(
    batch: &DomainChangeBatch,
) -> Result<Vec<CoalescedSelectiveRefresh>, DomainChangeStoreError> {
    batch.validate()?;
    let mut groups: BTreeMap<(String, DomainChangeSource), Group> = BTreeMap::new();
    let mut delivery_units = Vec::new();
    let mut last_sequence = None;

    for stored in batch.changes() {
        let envelope = stored.envelope();
        let key = (envelope.tenant_id().to_string(), envelope.source().clone());
        let can_accept = groups
            .get(&key)
            .is_none_or(|group| group.can_accept(envelope));
        if !can_accept {
            delivery_units.push(flush_groups(
                std::mem::take(&mut groups),
                last_sequence.expect("a full group has a validated prior sequence"),
            )?);
        }
        groups
            .entry(key)
            .or_insert_with(|| Group::new(envelope.projection().clone()))
            .add(stored.sequence(), envelope);
        last_sequence = Some(stored.sequence());
    }

    if !groups.is_empty() {
        delivery_units.push(flush_groups(
            groups,
            last_sequence.expect("a non-empty group has a validated sequence"),
        )?);
    }
    Ok(delivery_units)
}

fn flush_groups(
    groups: BTreeMap<(String, DomainChangeSource), Group>,
    through_sequence: u64,
) -> Result<CoalescedSelectiveRefresh, DomainChangeStoreError> {
    let notices = groups
        .into_iter()
        .map(|((tenant_id, source), group)| group.into_notice(tenant_id, source))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CoalescedSelectiveRefresh {
        through_sequence,
        notices,
    })
}

struct Group {
    watermark: DomainProjectionWatermark,
    targets: BTreeMap<DomainInvalidationTarget, TargetState>,
    event_ids: BTreeSet<String>,
    correlation_ids: BTreeSet<String>,
}

impl Group {
    fn new(watermark: DomainProjectionWatermark) -> Self {
        Self {
            watermark,
            targets: BTreeMap::new(),
            event_ids: BTreeSet::new(),
            correlation_ids: BTreeSet::new(),
        }
    }

    fn can_accept(&self, envelope: &DomainChangeEnvelope) -> bool {
        let target_count = self
            .targets
            .keys()
            .chain(envelope.targets())
            .collect::<BTreeSet<_>>()
            .len();
        let event_count =
            self.event_ids.len() + usize::from(!self.event_ids.contains(envelope.event_id()));
        let correlation_count = self.correlation_ids.len()
            + usize::from(!self.correlation_ids.contains(envelope.correlation_id()));
        target_count <= MAX_NOTICE_VALUES
            && event_count <= MAX_NOTICE_VALUES
            && correlation_count <= MAX_NOTICE_VALUES
    }

    fn add(&mut self, sequence: u64, envelope: &DomainChangeEnvelope) {
        if envelope.projection().revision() > self.watermark.revision()
            || envelope.projection().revision() == self.watermark.revision()
                && envelope.projection().observed_at() > self.watermark.observed_at()
        {
            self.watermark = envelope.projection().clone();
        }
        self.event_ids.insert(envelope.event_id().to_string());
        self.correlation_ids
            .insert(envelope.correlation_id().to_string());
        for target in envelope.targets() {
            let candidate = TargetState {
                sequence,
                reason: envelope.change_kind(),
                revision: envelope.projection().revision(),
            };
            self.targets
                .entry(target.clone())
                .and_modify(|current| {
                    if (candidate.revision, candidate.sequence)
                        > (current.revision, current.sequence)
                    {
                        *current = candidate.clone();
                    }
                })
                .or_insert(candidate);
        }
    }

    fn into_notice(
        self,
        tenant_id: String,
        source: DomainChangeSource,
    ) -> Result<SelectiveRefreshNotice, DomainChangeStoreError> {
        let targets = self
            .targets
            .into_iter()
            .map(|(target, state)| {
                SelectiveRefreshTarget::try_new(target, state.reason, state.revision)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(SelectiveRefreshNotice::try_new(
            tenant_id,
            source,
            self.watermark,
            targets,
            self.event_ids.into_iter().collect(),
            self.correlation_ids.into_iter().collect(),
        )?)
    }
}

#[derive(Clone)]
struct TargetState {
    sequence: u64,
    reason: DomainChangeKind,
    revision: u64,
}
