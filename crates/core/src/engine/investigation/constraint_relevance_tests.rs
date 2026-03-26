use crate::engine::investigation::constraint_relevance::retain_relevant_constraints;
use crate::model::{ConceptSeedKind, ConstraintEvidence};

#[test]
fn query_constraint_filter_prefers_column_level_matches_over_file_level_noise() {
    let retained = retain_relevant_constraints(
        "labs subject number uniqueness",
        ConceptSeedKind::Query,
        vec![
            ConstraintEvidence::new(
                "index_constraint",
                "index_declaration",
                "backend/alembic/versions/007_add_max_grade_to_labs.py".to_string(),
                32,
                32,
                "op.create_index(op.f('ix_users_group_id'), 'users', ['group_id'], unique=False)"
                    .to_string(),
                "strong",
                "backend/alembic/versions/007_add_max_grade_to_labs.py".to_string(),
                Some("backend/alembic/versions/007_add_max_grade_to_labs.py".to_string()),
                0.95,
                "op.create_index(op.f('ix_users_group_id'), 'users', ['group_id'], unique=False)"
                    .to_string(),
            ),
            ConstraintEvidence::new(
                "index_constraint",
                "index_declaration",
                "backend/alembic/versions/038_extend_labs_submissions.py".to_string(),
                44,
                44,
                "op.create_index('idx_labs_subject_number', 'labs', ['subject_id', 'number'])"
                    .to_string(),
                "strong",
                "backend/alembic/versions/038_extend_labs_submissions.py".to_string(),
                Some("backend/alembic/versions/038_extend_labs_submissions.py".to_string()),
                0.95,
                "op.create_index('idx_labs_subject_number', 'labs', ['subject_id', 'number'])"
                    .to_string(),
            ),
        ],
    );

    assert_eq!(retained.len(), 1);
    assert!(retained[0].excerpt.contains("idx_labs_subject_number"));
}

#[test]
fn query_constraint_filter_drops_single_token_noise_when_multi_token_hits_exist() {
    let retained = retain_relevant_constraints(
        "labs subject number uniqueness",
        ConceptSeedKind::Query,
        vec![
            ConstraintEvidence::new(
                "index_constraint",
                "index_declaration",
                "backend/alembic/versions/038_extend_labs_submissions.py".to_string(),
                36,
                36,
                "op.create_index('ix_labs_subject_id', 'labs', ['subject_id'])".to_string(),
                "strong",
                "backend/alembic/versions/038_extend_labs_submissions.py".to_string(),
                Some("backend/alembic/versions/038_extend_labs_submissions.py".to_string()),
                0.88,
                "op.create_index('ix_labs_subject_id', 'labs', ['subject_id'])".to_string(),
            ),
            ConstraintEvidence::new(
                "index_constraint",
                "index_declaration",
                "backend/alembic/versions/040_add_lab_publish_fields.py".to_string(),
                23,
                23,
                "op.create_index('ix_labs_public_code', 'labs', ['public_code'], unique=True)"
                    .to_string(),
                "strong",
                "backend/alembic/versions/040_add_lab_publish_fields.py".to_string(),
                Some("backend/alembic/versions/040_add_lab_publish_fields.py".to_string()),
                0.92,
                "op.create_index('ix_labs_public_code', 'labs', ['public_code'], unique=True)"
                    .to_string(),
            ),
            ConstraintEvidence::new(
                "index_constraint",
                "index_declaration",
                "backend/alembic/versions/038_extend_labs_submissions.py".to_string(),
                62,
                62,
                "op.create_index('idx_labs_subject_number', 'labs', ['subject_id', 'number'])"
                    .to_string(),
                "strong",
                "backend/alembic/versions/038_extend_labs_submissions.py".to_string(),
                Some("backend/alembic/versions/038_extend_labs_submissions.py".to_string()),
                0.95,
                "op.create_index('idx_labs_subject_number', 'labs', ['subject_id', 'number'])"
                    .to_string(),
            ),
        ],
    );

    assert_eq!(retained.len(), 2);
    assert!(retained[0].excerpt.contains("idx_labs_subject_number"));
    assert!(
        retained
            .iter()
            .all(|constraint| !constraint.excerpt.contains("ix_labs_public_code"))
    );
}

#[test]
fn query_constraint_filter_drops_constraints_without_any_query_overlap() {
    let retained = retain_relevant_constraints(
        "labs subject number uniqueness",
        ConceptSeedKind::Query,
        vec![
            ConstraintEvidence::new(
                "index_constraint",
                "index_declaration",
                "backend/alembic/versions/030_add_group_reports.py".to_string(),
                59,
                59,
                "op.create_index('idx_group_reports_code', 'group_reports', ['code'], unique=True)"
                    .to_string(),
                "strong",
                "backend/app/services/reports/attendance_helpers.py".to_string(),
                Some("backend/alembic/versions/030_add_group_reports.py".to_string()),
                0.95,
                "op.create_index('idx_group_reports_code', 'group_reports', ['code'], unique=True)"
                    .to_string(),
            ),
            ConstraintEvidence::new(
                "migration_constraint",
                "migration_declaration",
                "backend/alembic/versions/010_attestation_settings.py".to_string(),
                54,
                54,
                "sa.ForeignKeyConstraint(['group_id'], ['groups.id'], ondelete='CASCADE'),"
                    .to_string(),
                "strong",
                "backend/app/services/attestation/settings.py".to_string(),
                Some("backend/alembic/versions/010_attestation_settings.py".to_string()),
                0.92,
                "sa.ForeignKeyConstraint(['group_id'], ['groups.id'], ondelete='CASCADE'),"
                    .to_string(),
            ),
        ],
    );

    assert!(retained.is_empty());
}
