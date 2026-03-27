use std::fs;
use std::path::Path;

pub(super) const BACKEND_API_PATH: &str = "backend/app/api/v1/routes.py";
pub(super) const BACKEND_SERVICE_PATH: &str = "backend/app/services/auth/service.py";
pub(super) const FRONTEND_PAGE_PATH: &str = "frontend/src/app/login/page.tsx";
pub(super) const TEST_PATH: &str = "tests/test_auth_flow.py";
pub(super) const DOMAIN_PATH: &str = "domain/rules.py";
pub(super) const ORCHESTRATION_PATH: &str = "orchestration/pipeline.py";
pub(super) const API_PATH: &str = "api/routes.py";
pub(super) const SERVICE_PATH: &str = "services/translation_service.py";
pub(super) const MOD_ALPHA_ENTRY_PATH: &str =
    "mods/alpha_mod/src/main/java/dev/example/AlphaMod.java";
pub(super) const MOD_ALPHA_NETWORK_PATH: &str =
    "mods/alpha_mod/src/main/java/dev/example/network/AlphaNetworking.java";
pub(super) const MOD_ALPHA_PACKET_PATH: &str =
    "mods/alpha_mod/src/main/java/dev/example/network/AlphaPacket.java";
pub(super) const MOD_BETA_ENTRY_PATH: &str =
    "mods/beta_mod/src/client/java/dev/example/BetaClient.java";
pub(super) const MOD_BETA_MIXIN_PATH: &str =
    "mods/beta_mod/src/client/java/dev/example/mixin/BetaClientMixin.java";
pub(super) const MOD_BETA_CONFIG_PATH: &str =
    "mods/beta_mod/src/client/java/dev/example/config/BetaMixinConfig.java";
pub(super) const MOD_BETA_MODULE_PATH: &str =
    "mods/beta_mod/src/client/java/dev/example/modules/ModuleVision.java";

pub(super) fn write_bootstrap_broad_fixture(project_dir: &Path) {
    for relative in [
        BACKEND_API_PATH,
        BACKEND_SERVICE_PATH,
        FRONTEND_PAGE_PATH,
        TEST_PATH,
        DOMAIN_PATH,
        ORCHESTRATION_PATH,
        API_PATH,
        SERVICE_PATH,
        MOD_ALPHA_ENTRY_PATH,
        MOD_ALPHA_NETWORK_PATH,
        MOD_ALPHA_PACKET_PATH,
        MOD_BETA_ENTRY_PATH,
        MOD_BETA_MIXIN_PATH,
        MOD_BETA_CONFIG_PATH,
        MOD_BETA_MODULE_PATH,
        "docs/overview.md",
        ".ai/context/bootstrap_report.json",
    ] {
        let path = project_dir.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create fixture parent");
        }
    }

    fs::write(
        project_dir.join(BACKEND_API_PATH),
        "def login_entrypoint():\n    return 'backend api auth entrypoint route'\n",
    )
    .expect("write backend api");
    fs::write(
        project_dir.join(BACKEND_SERVICE_PATH),
        "def auth_service():\n    return 'backend auth service token session'\n",
    )
    .expect("write backend service");
    fs::write(
        project_dir.join(FRONTEND_PAGE_PATH),
        "export default function LoginPage(){ return 'frontend login page entrypoint' }\n",
    )
    .expect("write frontend page");
    fs::write(
        project_dir.join(TEST_PATH),
        "def test_auth_contract():\n    return 'auth contract test'\n",
    )
    .expect("write tests");
    fs::write(
        project_dir.join(DOMAIN_PATH),
        "def domain_rules():\n    return 'domain rules policy validation'\n",
    )
    .expect("write domain");
    fs::write(
        project_dir.join(ORCHESTRATION_PATH),
        "def orchestrate_translation_pipeline():\n    return 'orchestration workflow pipeline service'\n",
    )
    .expect("write orchestration");
    fs::write(
        project_dir.join(API_PATH),
        "def api_routes():\n    return 'api routes request boundary'\n",
    )
    .expect("write api");
    fs::write(
        project_dir.join(SERVICE_PATH),
        "def translation_service():\n    return 'service layer translation workflow'\n",
    )
    .expect("write service");
    fs::write(
        project_dir.join(MOD_ALPHA_ENTRY_PATH),
        "class AlphaMod { String v = \"mod entrypoint bootstrap runtime\"; }\n",
    )
    .expect("write alpha mod");
    fs::write(
        project_dir.join(MOD_ALPHA_NETWORK_PATH),
        "class AlphaNetworking { String v = \"mod runtime hooks network packet\"; }\n",
    )
    .expect("write alpha network");
    fs::write(
        project_dir.join(MOD_ALPHA_PACKET_PATH),
        "class AlphaPacket { String v = \"mod runtime network packet\"; }\n",
    )
    .expect("write alpha packet");
    fs::write(
        project_dir.join(MOD_BETA_ENTRY_PATH),
        "class BetaClient { String v = \"mod client entrypoint runtime\"; }\n",
    )
    .expect("write beta entry");
    fs::write(
        project_dir.join(MOD_BETA_MIXIN_PATH),
        "class BetaClientMixin { String v = \"mod mixins hooks runtime\"; }\n",
    )
    .expect("write beta mixin");
    fs::write(
        project_dir.join(MOD_BETA_CONFIG_PATH),
        "class BetaMixinConfig { String v = \"mod mixins runtime config\"; }\n",
    )
    .expect("write beta config");
    fs::write(
        project_dir.join(MOD_BETA_MODULE_PATH),
        "class ModuleVision { String v = \"mod modules render client config\"; }\n",
    )
    .expect("write beta module");
    fs::write(
        project_dir.join("docs/overview.md"),
        "system overview architecture map\n",
    )
    .expect("write docs");
    fs::write(
        project_dir.join(".ai/context/bootstrap_report.json"),
        "{\"kind\":\"analysis\"}\n",
    )
    .expect("write ai artifact");
}
