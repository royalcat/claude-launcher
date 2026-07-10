/// How a value stored inside `CLAUDE_CODE_EXTRA_BODY` is serialized/deserialized.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ExtraBodyValueType {
    /// Plain text → JSON string
    String,
    /// Comma-separated input → JSON array of strings
    StringList,
    /// "true"/"false" text → JSON bool
    Bool,
}

/// The type of a provider field, determining how it's rendered in forms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldType {
    Url,
    Secret,
    String,
    /// Value is stored inside `CLAUDE_CODE_EXTRA_BODY` at a dot-separated JSON path.
    ExtraBody {
        json_path: &'static str,
        value_type: ExtraBodyValueType,
    },
    /// User picks from predefined options; "Custom..." is appended at render time.
    Choice { options: &'static [&'static str] },
}

/// A single field in a provider's profile form.
#[derive(Debug, Clone)]
pub struct ProviderField {
    /// The environment variable key (e.g. `ANTHROPIC_AUTH_TOKEN`)
    pub key: &'static str,
    /// Human-readable label shown in forms
    pub label: &'static str,
    pub field_type: FieldType,
    pub required: bool,
    pub default: Option<&'static str>,
}

/// A provider definition: describes what env vars and config Claude Code needs
/// to talk to a specific API endpoint.
#[derive(Debug, Clone)]
pub struct ProviderDef {
    pub id: &'static str,
    pub name: &'static str,
    pub fields: &'static [ProviderField],
    pub supports_statusline: bool,
}

// ---- Provider definitions ------------------------------------------------

macro_rules! field {
    ($key:expr, $label:expr, url, required) => {
        ProviderField {
            key: $key,
            label: $label,
            field_type: FieldType::Url,
            required: true,
            default: None,
        }
    };
    ($key:expr, $label:expr, url, required, $default:expr) => {
        ProviderField {
            key: $key,
            label: $label,
            field_type: FieldType::Url,
            required: true,
            default: Some($default),
        }
    };
    ($key:expr, $label:expr, secret, required) => {
        ProviderField {
            key: $key,
            label: $label,
            field_type: FieldType::Secret,
            required: true,
            default: None,
        }
    };
    ($key:expr, $label:expr, secret, required, $default:expr) => {
        ProviderField {
            key: $key,
            label: $label,
            field_type: FieldType::Secret,
            required: true,
            default: Some($default),
        }
    };
    ($key:expr, $label:expr, secret, optional) => {
        ProviderField {
            key: $key,
            label: $label,
            field_type: FieldType::Secret,
            required: false,
            default: None,
        }
    };
    ($key:expr, $label:expr, string, optional) => {
        ProviderField {
            key: $key,
            label: $label,
            field_type: FieldType::String,
            required: false,
            default: None,
        }
    };
    ($key:expr, $label:expr, extra_body_string($path:expr), optional) => {
        ProviderField {
            key: $key,
            label: $label,
            field_type: FieldType::ExtraBody {
                json_path: $path,
                value_type: ExtraBodyValueType::String,
            },
            required: false,
            default: None,
        }
    };
    ($key:expr, $label:expr, extra_body_string_list($path:expr), optional) => {
        ProviderField {
            key: $key,
            label: $label,
            field_type: FieldType::ExtraBody {
                json_path: $path,
                value_type: ExtraBodyValueType::StringList,
            },
            required: false,
            default: None,
        }
    };
    ($key:expr, $label:expr, extra_body_bool($path:expr), optional) => {
        ProviderField {
            key: $key,
            label: $label,
            field_type: FieldType::ExtraBody {
                json_path: $path,
                value_type: ExtraBodyValueType::Bool,
            },
            required: false,
            default: None,
        }
    };
    ($key:expr, $label:expr, choice($options:expr), optional) => {
        ProviderField {
            key: $key,
            label: $label,
            field_type: FieldType::Choice { options: $options },
            required: false,
            default: None,
        }
    };
    ($key:expr, $label:expr, choice($options:expr), optional, $default:expr) => {
        ProviderField {
            key: $key,
            label: $label,
            field_type: FieldType::Choice { options: $options },
            required: false,
            default: Some($default),
        }
    };
    ($key:expr, $label:expr, url, optional) => {
        ProviderField {
            key: $key,
            label: $label,
            field_type: FieldType::Url,
            required: false,
            default: None,
        }
    };
    ($key:expr, $label:expr, url, optional, $default:expr) => {
        ProviderField {
            key: $key,
            label: $label,
            field_type: FieldType::Url,
            required: false,
            default: Some($default),
        }
    };
}

// Helper: model override fields included in each provider's static slice
// using a const fn that concatenates two slices. Since Rust doesn't yet have
// const-friendly Vec, we define each provider's full field list explicitly.

const ENV_BASE_URL: &str = "ANTHROPIC_BASE_URL";
const ENV_AUTH_TOKEN: &str = "ANTHROPIC_AUTH_TOKEN";
const ENV_HAIKU_MODEL: &str = "ANTHROPIC_DEFAULT_HAIKU_MODEL";
const ENV_SONNET_MODEL: &str = "ANTHROPIC_DEFAULT_SONNET_MODEL";
const ENV_OPUS_MODEL: &str = "ANTHROPIC_DEFAULT_OPUS_MODEL";
pub const ENV_EXTRA_BODY: &str = "CLAUDE_CODE_EXTRA_BODY";
pub const ENV_EFFORT_LEVEL: &str = "CLAUDE_CODE_EFFORT_LEVEL";
/// Options for the CLAUDE_CODE_EFFORT_LEVEL environment variable.
/// "Custom..." is added at render time and is not in this list.
pub const EFFORT_LEVEL_OPTIONS: &[&str] = &["auto", "low", "medium", "high", "xhigh", "max"];

static ANTHROPIC_COMPATIBLE_FIELDS: &[ProviderField] = &[
    field!(ENV_BASE_URL, "Anthropic Compatible URL", url, required, "http://localhost:8083"),
    field!(ENV_AUTH_TOKEN, "Auth Token", secret, optional),
    field!(ENV_HAIKU_MODEL, "Haiku Model Override", string, optional),
    field!(ENV_SONNET_MODEL, "Sonnet Model Override", string, optional),
    field!(ENV_OPUS_MODEL, "Opus Model Override", string, optional),
    field!(ENV_EFFORT_LEVEL, "Effort Level", choice(EFFORT_LEVEL_OPTIONS), optional),
];

static OPENROUTER_FIELDS: &[ProviderField] = &[
    field!(ENV_BASE_URL, "API Base URL", url, required, "https://openrouter.ai/api"),
    field!(ENV_AUTH_TOKEN, "API Key", secret, required),
    field!(ENV_HAIKU_MODEL, "Haiku Model Override", string, optional),
    field!(ENV_SONNET_MODEL, "Sonnet Model Override", string, optional),
    field!(ENV_OPUS_MODEL, "Opus Model Override", string, optional),
    field!(
        ENV_EXTRA_BODY,
        "Provider Only (comma-separated)",
        extra_body_string_list("provider.only"),
        optional
    ),
    field!(
        ENV_EXTRA_BODY,
        "Allow Fallbacks",
        extra_body_bool("provider.allow_fallbacks"),
        optional
    ),
    field!(ENV_EFFORT_LEVEL, "Effort Level", choice(EFFORT_LEVEL_OPTIONS), optional),
];

static ZAI_FIELDS: &[ProviderField] = &[
    field!(ENV_BASE_URL, "API Base URL", url, required, "https://api.z.ai/api/anthropic"),
    field!(ENV_AUTH_TOKEN, "API Key", secret, required),
    field!(ENV_HAIKU_MODEL, "Haiku Model Override", string, optional),
    field!(ENV_SONNET_MODEL, "Sonnet Model Override", string, optional),
    field!(ENV_OPUS_MODEL, "Opus Model Override", string, optional),
    field!(ENV_EFFORT_LEVEL, "Effort Level", choice(EFFORT_LEVEL_OPTIONS), optional),
];

static DEEPSEEK_FIELDS: &[ProviderField] = &[
    field!(ENV_BASE_URL, "API Base URL", url, required, "https://api.deepseek.com/anthropic"),
    field!(ENV_AUTH_TOKEN, "API Key", secret, required),
    field!(ENV_HAIKU_MODEL, "Haiku Model Override", string, optional),
    field!(ENV_SONNET_MODEL, "Sonnet Model Override", string, optional),
    field!(ENV_OPUS_MODEL, "Opus Model Override", string, optional),
    field!(ENV_EFFORT_LEVEL, "Effort Level", choice(EFFORT_LEVEL_OPTIONS), optional, "max"),
];

static MINIMAX_FIELDS: &[ProviderField] = &[
    field!(ENV_BASE_URL, "API Base URL", url, required, "https://api.minimax.chat/v1"),
    field!(ENV_AUTH_TOKEN, "API Key", secret, required),
    field!(ENV_HAIKU_MODEL, "Haiku Model Override", string, optional),
    field!(ENV_SONNET_MODEL, "Sonnet Model Override", string, optional),
    field!(ENV_OPUS_MODEL, "Opus Model Override", string, optional),
    field!(ENV_EFFORT_LEVEL, "Effort Level", choice(EFFORT_LEVEL_OPTIONS), optional),
];

static GLM_FIELDS: &[ProviderField] = &[
    field!(ENV_BASE_URL, "API Base URL", url, required, "https://open.bigmodel.cn/api/anthropic"),
    field!(ENV_AUTH_TOKEN, "API Key", secret, required),
    field!(ENV_HAIKU_MODEL, "Haiku Model Override", string, optional),
    field!(ENV_SONNET_MODEL, "Sonnet Model Override", string, optional),
    field!(ENV_OPUS_MODEL, "Opus Model Override", string, optional),
    field!(ENV_EFFORT_LEVEL, "Effort Level", choice(EFFORT_LEVEL_OPTIONS), optional),
];

static MOONSHOT_FIELDS: &[ProviderField] = &[
    field!(ENV_BASE_URL, "API Base URL", url, required, "https://api.moonshot.cn/anthropic"),
    field!(ENV_AUTH_TOKEN, "API Key", secret, required),
    field!(ENV_HAIKU_MODEL, "Haiku Model Override", string, optional),
    field!(ENV_SONNET_MODEL, "Sonnet Model Override", string, optional),
    field!(ENV_OPUS_MODEL, "Opus Model Override", string, optional),
    field!(ENV_EFFORT_LEVEL, "Effort Level", choice(EFFORT_LEVEL_OPTIONS), optional),
];

static QWEN_FIELDS: &[ProviderField] = &[
    field!(
        ENV_BASE_URL,
        "API Base URL",
        url,
        required,
        "https://dashscope.aliyuncs.com/compatible-mode/v1"
    ),
    field!(ENV_AUTH_TOKEN, "API Key", secret, required),
    field!(ENV_HAIKU_MODEL, "Haiku Model Override", string, optional),
    field!(ENV_SONNET_MODEL, "Sonnet Model Override", string, optional),
    field!(ENV_OPUS_MODEL, "Opus Model Override", string, optional),
    field!(ENV_EFFORT_LEVEL, "Effort Level", choice(EFFORT_LEVEL_OPTIONS), optional),
];

static FIREWORKS_FIELDS: &[ProviderField] = &[
    field!(ENV_BASE_URL, "API Base URL", url, required, "https://api.fireworks.ai/inference/v1"),
    field!(ENV_AUTH_TOKEN, "API Key", secret, required),
    field!(ENV_HAIKU_MODEL, "Haiku Model Override", string, optional),
    field!(ENV_SONNET_MODEL, "Sonnet Model Override", string, optional),
    field!(ENV_OPUS_MODEL, "Opus Model Override", string, optional),
    field!(ENV_EFFORT_LEVEL, "Effort Level", choice(EFFORT_LEVEL_OPTIONS), optional),
];

static VOLCENGINE_FIELDS: &[ProviderField] = &[
    field!(ENV_BASE_URL, "API Base URL", url, required, "https://ark.cn-beijing.volces.com/api/v3"),
    field!(ENV_AUTH_TOKEN, "API Key", secret, required),
    field!(ENV_HAIKU_MODEL, "Haiku Model Override", string, optional),
    field!(ENV_SONNET_MODEL, "Sonnet Model Override", string, optional),
    field!(ENV_OPUS_MODEL, "Opus Model Override", string, optional),
    field!(ENV_EFFORT_LEVEL, "Effort Level", choice(EFFORT_LEVEL_OPTIONS), optional),
];

static NVIDIA_NIM_FIELDS: &[ProviderField] = &[
    field!(ENV_BASE_URL, "API Base URL", url, required, "https://integrate.api.nvidia.com/v1"),
    field!(ENV_AUTH_TOKEN, "API Key", secret, required),
    field!(ENV_HAIKU_MODEL, "Haiku Model Override", string, optional),
    field!(ENV_SONNET_MODEL, "Sonnet Model Override", string, optional),
    field!(ENV_OPUS_MODEL, "Opus Model Override", string, optional),
    field!(ENV_EFFORT_LEVEL, "Effort Level", choice(EFFORT_LEVEL_OPTIONS), optional),
];

static OLLAMA_FIELDS: &[ProviderField] = &[
    field!(ENV_BASE_URL, "Ollama URL", url, required, "http://localhost:11434"),
    field!(ENV_AUTH_TOKEN, "Auth Token", secret, optional),
    field!(ENV_HAIKU_MODEL, "Haiku Model Override", string, optional),
    field!(ENV_SONNET_MODEL, "Sonnet Model Override", string, optional),
    field!(ENV_OPUS_MODEL, "Opus Model Override", string, optional),
    field!(ENV_EFFORT_LEVEL, "Effort Level", choice(EFFORT_LEVEL_OPTIONS), optional),
];

static LMSTUDIO_FIELDS: &[ProviderField] = &[
    field!(ENV_BASE_URL, "LM Studio URL", url, required, "http://localhost:1234"),
    field!(ENV_AUTH_TOKEN, "Auth Token", secret, required, "lm-studio"),
    field!(ENV_HAIKU_MODEL, "Haiku Model Override", string, optional),
    field!(ENV_SONNET_MODEL, "Sonnet Model Override", string, optional),
    field!(ENV_OPUS_MODEL, "Opus Model Override", string, optional),
    field!(ENV_EFFORT_LEVEL, "Effort Level", choice(EFFORT_LEVEL_OPTIONS), optional),
];

static VLLM_FIELDS: &[ProviderField] = &[
    field!(ENV_BASE_URL, "vLLM URL", url, required, "http://localhost:8000"),
    field!(ENV_AUTH_TOKEN, "Auth Token", secret, optional),
    field!(ENV_HAIKU_MODEL, "Haiku Model Override", string, optional),
    field!(ENV_SONNET_MODEL, "Sonnet Model Override", string, optional),
    field!(ENV_OPUS_MODEL, "Opus Model Override", string, optional),
    field!(ENV_EFFORT_LEVEL, "Effort Level", choice(EFFORT_LEVEL_OPTIONS), optional),
];

static LITELLM_FIELDS: &[ProviderField] = &[
    field!(ENV_BASE_URL, "LiteLLM URL", url, required, "http://localhost:4000"),
    field!(ENV_AUTH_TOKEN, "API Key", secret, optional),
    field!(ENV_HAIKU_MODEL, "Haiku Model Override", string, optional),
    field!(ENV_SONNET_MODEL, "Sonnet Model Override", string, optional),
    field!(ENV_OPUS_MODEL, "Opus Model Override", string, optional),
    field!(ENV_EFFORT_LEVEL, "Effort Level", choice(EFFORT_LEVEL_OPTIONS), optional),
];

static CLOUDFLARE_FIELDS: &[ProviderField] = &[
    field!(ENV_BASE_URL, "Gateway URL", url, required),
    field!(ENV_AUTH_TOKEN, "API Key", secret, required),
    field!(ENV_HAIKU_MODEL, "Haiku Model Override", string, optional),
    field!(ENV_SONNET_MODEL, "Sonnet Model Override", string, optional),
    field!(ENV_OPUS_MODEL, "Opus Model Override", string, optional),
    field!(ENV_EFFORT_LEVEL, "Effort Level", choice(EFFORT_LEVEL_OPTIONS), optional),
];

static VERCEL_FIELDS: &[ProviderField] = &[
    field!(ENV_BASE_URL, "Gateway URL", url, required),
    field!(ENV_AUTH_TOKEN, "API Key", secret, required),
    field!(ENV_HAIKU_MODEL, "Haiku Model Override", string, optional),
    field!(ENV_SONNET_MODEL, "Sonnet Model Override", string, optional),
    field!(ENV_OPUS_MODEL, "Opus Model Override", string, optional),
    field!(ENV_EFFORT_LEVEL, "Effort Level", choice(EFFORT_LEVEL_OPTIONS), optional),
];

// ---- Registry ------------------------------------------------------------

pub static PROVIDERS: &[ProviderDef] = &[
    ProviderDef {
        id: "anthropic-compatible",
        name: "Any Anthropic or OpenAI Compatible",
        fields: ANTHROPIC_COMPATIBLE_FIELDS,
        supports_statusline: false,
    },
    ProviderDef {
        id: "zai",
        name: "z.ai (GLM)",
        fields: ZAI_FIELDS,
        supports_statusline: false,
    },
    ProviderDef {
        id: "openrouter",
        name: "OpenRouter",
        fields: OPENROUTER_FIELDS,
        supports_statusline: false,
    },
    ProviderDef {
        id: "deepseek",
        name: "DeepSeek",
        fields: DEEPSEEK_FIELDS,
        supports_statusline: true,
    },
    ProviderDef {
        id: "minimax",
        name: "MiniMax",
        fields: MINIMAX_FIELDS,
        supports_statusline: false,
    },
    ProviderDef {
        id: "glm",
        name: "GLM (ZhipuAI)",
        fields: GLM_FIELDS,
        supports_statusline: false,
    },
    ProviderDef {
        id: "moonshot",
        name: "Moonshot",
        fields: MOONSHOT_FIELDS,
        supports_statusline: false,
    },
    ProviderDef {
        id: "qwen",
        name: "Qwen (Alibaba)",
        fields: QWEN_FIELDS,
        supports_statusline: false,
    },
    ProviderDef {
        id: "fireworks",
        name: "Fireworks AI",
        fields: FIREWORKS_FIELDS,
        supports_statusline: false,
    },
    ProviderDef {
        id: "volcengine",
        name: "Volcengine",
        fields: VOLCENGINE_FIELDS,
        supports_statusline: false,
    },
    ProviderDef {
        id: "nvidia-nim",
        name: "NVIDIA NIM",
        fields: NVIDIA_NIM_FIELDS,
        supports_statusline: false,
    },
    ProviderDef {
        id: "ollama",
        name: "Ollama (local)",
        fields: OLLAMA_FIELDS,
        supports_statusline: false,
    },
    ProviderDef {
        id: "lmstudio",
        name: "LM Studio",
        fields: LMSTUDIO_FIELDS,
        supports_statusline: false,
    },
    ProviderDef {
        id: "vllm",
        name: "vLLM",
        fields: VLLM_FIELDS,
        supports_statusline: false,
    },
    ProviderDef {
        id: "litellm",
        name: "LiteLLM",
        fields: LITELLM_FIELDS,
        supports_statusline: false,
    },
    ProviderDef {
        id: "cloudflare",
        name: "Cloudflare AI Gateway",
        fields: CLOUDFLARE_FIELDS,
        supports_statusline: false,
    },
    ProviderDef {
        id: "vercel",
        name: "Vercel AI Gateway",
        fields: VERCEL_FIELDS,
        supports_statusline: false,
    },
];

pub fn get_provider(id: &str) -> Option<&'static ProviderDef> {
    PROVIDERS.iter().find(|p| p.id == id)
}
