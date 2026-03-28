use crate::quality::duplication::artifact::DuplicationSignalRole;
use crate::quality::duplication::surface::signal_token_floor_for_surface;

use super::classify_signal_role;

#[test]
fn classifies_python_models_as_boilerplate() {
    let classification = classify_signal_role(
        "python",
        &[
            "app/models/user_schema.py".to_string(),
            "app/models/admin_schema.py".to_string(),
        ],
        &[
            "$attr",
            "dataclass",
            "class",
            "$id",
            ":",
            "$id",
            ":",
            "$id",
            "$id",
            ":",
            "$id",
            "$id",
            ":",
            "$id",
            "$id",
            "=",
            "$lit",
            "$id",
            ":",
            "$id",
            "$id",
            "=",
            "$lit",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>(),
    );

    assert_eq!(classification.role, DuplicationSignalRole::Boilerplate);
}

#[test]
fn classifies_java_annotation_wrappers_as_boilerplate() {
    let classification = classify_signal_role(
        "java",
        &[
            "src/main/java/app/admin/UserConfigView.java".to_string(),
            "src/main/java/app/admin/AdminConfigView.java".to_string(),
        ],
        &[
            "$attr",
            "$attr",
            "configurationproperties",
            "public",
            "class",
            "$id",
            "{",
            "private",
            "$id",
            "$id",
            ";",
            "private",
            "$id",
            "$id",
            ";",
            "private",
            "$id",
            "$id",
            ";",
            "private",
            "$id",
            "$id",
            ";",
            "}",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>(),
    );

    assert_eq!(classification.role, DuplicationSignalRole::Boilerplate);
}

#[test]
fn mixed_model_and_service_paths_stay_primary() {
    let classification = classify_signal_role(
        "python",
        &[
            "app/models/user_schema.py".to_string(),
            "app/services/user_projection.py".to_string(),
        ],
        &[
            "$attr",
            "dataclass",
            "class",
            "$id",
            ":",
            "field",
            "(",
            "$lit",
            ")",
            "$id",
            ":",
            "$id",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>(),
    );

    assert_eq!(classification.role, DuplicationSignalRole::Primary);
}

#[test]
fn mixed_imperative_paths_stay_primary() {
    let classification = classify_signal_role(
        "python",
        &[
            "app/models/user_projection.py".to_string(),
            "app/services/user_projection.py".to_string(),
        ],
        &[
            "def", "$id", "(", "$id", ":", "$id", ")", "->", "$id", ":", "$id", "=", "$id", "$id",
            "+=", "$num", "$id", "+=", "$num", "$id", "+=", "$num", "$id", "+=", "$num", "return",
            "$id",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>(),
    );

    assert_eq!(classification.role, DuplicationSignalRole::Primary);
}

#[test]
fn wrapper_surfaces_raise_signal_floor() {
    assert_eq!(
        signal_token_floor_for_surface(
            "src/components/admin/AuditCard.tsx",
            "tsx",
            DuplicationSignalRole::Downweighted,
        ),
        256
    );
}
