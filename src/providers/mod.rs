pub mod llamacpp;

pub use llamacpp::PROVIDER_LLAMACPP_SINGLE_MODEL;

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
    /// "128" integer text → JSON number
    Number,
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
    Choice {
        options: &'static [&'static str],
    },
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

impl ProviderField {
    /// Start building a field. `field_type` is a mandatory argument; the
    /// requirement and default are set by the chained modifiers below.
    /// Defaults to optional with no default value.
    pub const fn field(key: &'static str, label: &'static str, field_type: FieldType) -> Self {
        ProviderField {
            key,
            label,
            field_type,
            required: false,
            default: None,
        }
    }

    pub const fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub const fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    pub const fn default(mut self, default: &'static str) -> Self {
        self.default = Some(default);
        self
    }
}

/// A provider definition: describes what env vars and config Claude Code needs
/// to talk to a specific API endpoint.
#[derive(Debug, Clone)]
pub struct ProviderDef {
    pub id: &'static str,
    pub name: &'static str,
    pub fields: &'static [ProviderField],
    pub supports_statusline: bool,
    /// Optional grouping of `fields` into tabs for the form. `None` renders a flat form.
    pub groups: Option<&'static [ProviderFieldGroup]>,
}

impl ProviderDef {
    /// Number of tabs, or 1 when the provider has no groupings.
    pub fn tab_count(&self) -> usize {
        self.groups.map_or(1, |g| g.len())
    }

    /// Absolute field indices rendered by a given tab (clamped). With no groupings,
    /// returns every field index, so the form renders flat as before.
    pub fn tab_field_indices(&self, tab: usize) -> Vec<usize> {
        match self.groups {
            Some(groups) => groups
                .get(tab.min(groups.len().saturating_sub(1)))
                .map(|g| g.field_indices.to_vec())
                .unwrap_or_default(),
            None => (0..self.fields.len()).collect(),
        }
    }

    /// The group that owns a field index, if any.
    pub fn tab_for_field(&self, field_idx: usize) -> Option<usize> {
        self.groups?.iter().position(|g| g.field_indices.contains(&field_idx))
    }
}

/// A named group of provider fields, rendered as a tab in the form.
#[derive(Debug, Clone)]
pub struct ProviderFieldGroup {
    pub label: &'static str,
    /// Absolute indices into the parent `ProviderDef::fields`.
    pub field_indices: &'static [usize],
}

// ---- Provider definitions ------------------------------------------------

pub const ENV_BASE_URL: &str = "ANTHROPIC_BASE_URL";
pub const ENV_AUTH_TOKEN: &str = "ANTHROPIC_AUTH_TOKEN";
pub const ENV_HAIKU_MODEL: &str = "ANTHROPIC_DEFAULT_HAIKU_MODEL";
pub const ENV_SONNET_MODEL: &str = "ANTHROPIC_DEFAULT_SONNET_MODEL";
pub const ENV_OPUS_MODEL: &str = "ANTHROPIC_DEFAULT_OPUS_MODEL";
pub const ENV_EXTRA_BODY: &str = "CLAUDE_CODE_EXTRA_BODY";
pub const ENV_EFFORT_LEVEL: &str = "CLAUDE_CODE_EFFORT_LEVEL";
/// Launcher-internal: used only by the statusline to fetch the account balance.
/// Never exported to the launched `claude` process (see actions/launch.rs).
pub const ENV_MANAGEMENT_KEY: &str = "OPENROUTER_MANAGEMENT_KEY";
/// Options for the CLAUDE_CODE_EFFORT_LEVEL environment variable.
/// "Custom..." is added at render time and is not in this list.
pub const EFFORT_LEVEL_OPTIONS: &[&str] = &["auto", "low", "medium", "high", "xhigh", "max"];

static ANTHROPIC_COMPATIBLE_FIELDS: &[ProviderField] = &[
    ProviderField::field(ENV_BASE_URL, "Anthropic Compatible URL", FieldType::Url)
        .required()
        .default("http://localhost:8083"),
    ProviderField::field(ENV_AUTH_TOKEN, "Auth Token", FieldType::Secret).optional(),
    ProviderField::field(ENV_HAIKU_MODEL, "Haiku Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_SONNET_MODEL, "Sonnet Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_OPUS_MODEL, "Opus Model Override", FieldType::String).optional(),
    ProviderField::field(
        ENV_EFFORT_LEVEL,
        "Effort Level",
        FieldType::Choice {
            options: EFFORT_LEVEL_OPTIONS,
        },
    )
    .optional(),
];

static OPENROUTER_FIELDS: &[ProviderField] = &[
    ProviderField::field(ENV_BASE_URL, "API Base URL", FieldType::Url)
        .required()
        .default("https://openrouter.ai/api"),
    ProviderField::field(ENV_AUTH_TOKEN, "API Key", FieldType::Secret).required(),
    ProviderField::field(ENV_MANAGEMENT_KEY, "Management Key", FieldType::Secret).optional(),
    ProviderField::field(ENV_HAIKU_MODEL, "Haiku Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_SONNET_MODEL, "Sonnet Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_OPUS_MODEL, "Opus Model Override", FieldType::String).optional(),
    ProviderField::field(
        ENV_EXTRA_BODY,
        "Provider Only (comma-separated)",
        FieldType::ExtraBody {
            json_path: "provider.only",
            value_type: ExtraBodyValueType::StringList,
        },
    )
    .optional(),
    ProviderField::field(
        ENV_EXTRA_BODY,
        "Quantization Levels (comma-separated)",
        FieldType::ExtraBody {
            json_path: "provider.quantizations",
            value_type: ExtraBodyValueType::StringList,
        },
    )
    .optional(),
    ProviderField::field(
        ENV_EXTRA_BODY,
        "Min Throughput (tokens/s)",
        FieldType::ExtraBody {
            json_path: "provider.preferred_min_throughput",
            value_type: ExtraBodyValueType::Number,
        },
    )
    .optional(),
    ProviderField::field(
        ENV_EXTRA_BODY,
        "Allow Fallbacks",
        FieldType::ExtraBody {
            json_path: "provider.allow_fallbacks",
            value_type: ExtraBodyValueType::Bool,
        },
    )
    .optional(),
    ProviderField::field(
        ENV_EFFORT_LEVEL,
        "Effort Level",
        FieldType::Choice {
            options: EFFORT_LEVEL_OPTIONS,
        },
    )
    .optional(),
];

/// Tab groupings over `OPENROUTER_FIELDS` indices (see that field list above).
static OPENROUTER_GROUPS: &[ProviderFieldGroup] = &[
    ProviderFieldGroup {
        label: "General",
        field_indices: &[0, 1, 2, 3, 4, 5, 10],
    },
    ProviderFieldGroup {
        label: "Provider Selection",
        field_indices: &[6, 7, 8, 9],
    },
];

static ZAI_FIELDS: &[ProviderField] = &[
    ProviderField::field(ENV_BASE_URL, "API Base URL", FieldType::Url)
        .required()
        .default("https://api.z.ai/api/anthropic"),
    ProviderField::field(ENV_AUTH_TOKEN, "API Key", FieldType::Secret).required(),
    ProviderField::field(ENV_HAIKU_MODEL, "Haiku Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_SONNET_MODEL, "Sonnet Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_OPUS_MODEL, "Opus Model Override", FieldType::String).optional(),
    ProviderField::field(
        ENV_EFFORT_LEVEL,
        "Effort Level",
        FieldType::Choice {
            options: EFFORT_LEVEL_OPTIONS,
        },
    )
    .optional(),
];

static DEEPSEEK_FIELDS: &[ProviderField] = &[
    ProviderField::field(ENV_BASE_URL, "API Base URL", FieldType::Url)
        .required()
        .default("https://api.deepseek.com/anthropic"),
    ProviderField::field(ENV_AUTH_TOKEN, "API Key", FieldType::Secret).required(),
    ProviderField::field(ENV_HAIKU_MODEL, "Haiku Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_SONNET_MODEL, "Sonnet Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_OPUS_MODEL, "Opus Model Override", FieldType::String).optional(),
    ProviderField::field(
        ENV_EFFORT_LEVEL,
        "Effort Level",
        FieldType::Choice {
            options: EFFORT_LEVEL_OPTIONS,
        },
    )
    .optional()
    .default("max"),
];

static MINIMAX_FIELDS: &[ProviderField] = &[
    ProviderField::field(ENV_BASE_URL, "API Base URL", FieldType::Url)
        .required()
        .default("https://api.minimax.chat/v1"),
    ProviderField::field(ENV_AUTH_TOKEN, "API Key", FieldType::Secret).required(),
    ProviderField::field(ENV_HAIKU_MODEL, "Haiku Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_SONNET_MODEL, "Sonnet Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_OPUS_MODEL, "Opus Model Override", FieldType::String).optional(),
    ProviderField::field(
        ENV_EFFORT_LEVEL,
        "Effort Level",
        FieldType::Choice {
            options: EFFORT_LEVEL_OPTIONS,
        },
    )
    .optional(),
];

static GLM_FIELDS: &[ProviderField] = &[
    ProviderField::field(ENV_BASE_URL, "API Base URL", FieldType::Url)
        .required()
        .default("https://open.bigmodel.cn/api/anthropic"),
    ProviderField::field(ENV_AUTH_TOKEN, "API Key", FieldType::Secret).required(),
    ProviderField::field(ENV_HAIKU_MODEL, "Haiku Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_SONNET_MODEL, "Sonnet Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_OPUS_MODEL, "Opus Model Override", FieldType::String).optional(),
    ProviderField::field(
        ENV_EFFORT_LEVEL,
        "Effort Level",
        FieldType::Choice {
            options: EFFORT_LEVEL_OPTIONS,
        },
    )
    .optional(),
];

static MOONSHOT_FIELDS: &[ProviderField] = &[
    ProviderField::field(ENV_BASE_URL, "API Base URL", FieldType::Url)
        .required()
        .default("https://api.moonshot.cn/anthropic"),
    ProviderField::field(ENV_AUTH_TOKEN, "API Key", FieldType::Secret).required(),
    ProviderField::field(ENV_HAIKU_MODEL, "Haiku Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_SONNET_MODEL, "Sonnet Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_OPUS_MODEL, "Opus Model Override", FieldType::String).optional(),
    ProviderField::field(
        ENV_EFFORT_LEVEL,
        "Effort Level",
        FieldType::Choice {
            options: EFFORT_LEVEL_OPTIONS,
        },
    )
    .optional(),
];

static QWEN_FIELDS: &[ProviderField] = &[
    ProviderField::field(ENV_BASE_URL, "API Base URL", FieldType::Url)
        .required()
        .default("https://dashscope.aliyuncs.com/compatible-mode/v1"),
    ProviderField::field(ENV_AUTH_TOKEN, "API Key", FieldType::Secret).required(),
    ProviderField::field(ENV_HAIKU_MODEL, "Haiku Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_SONNET_MODEL, "Sonnet Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_OPUS_MODEL, "Opus Model Override", FieldType::String).optional(),
    ProviderField::field(
        ENV_EFFORT_LEVEL,
        "Effort Level",
        FieldType::Choice {
            options: EFFORT_LEVEL_OPTIONS,
        },
    )
    .optional(),
];

static FIREWORKS_FIELDS: &[ProviderField] = &[
    ProviderField::field(ENV_BASE_URL, "API Base URL", FieldType::Url)
        .required()
        .default("https://api.fireworks.ai/inference/v1"),
    ProviderField::field(ENV_AUTH_TOKEN, "API Key", FieldType::Secret).required(),
    ProviderField::field(ENV_HAIKU_MODEL, "Haiku Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_SONNET_MODEL, "Sonnet Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_OPUS_MODEL, "Opus Model Override", FieldType::String).optional(),
    ProviderField::field(
        ENV_EFFORT_LEVEL,
        "Effort Level",
        FieldType::Choice {
            options: EFFORT_LEVEL_OPTIONS,
        },
    )
    .optional(),
];

static VOLCENGINE_FIELDS: &[ProviderField] = &[
    ProviderField::field(ENV_BASE_URL, "API Base URL", FieldType::Url)
        .required()
        .default("https://ark.cn-beijing.volces.com/api/v3"),
    ProviderField::field(ENV_AUTH_TOKEN, "API Key", FieldType::Secret).required(),
    ProviderField::field(ENV_HAIKU_MODEL, "Haiku Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_SONNET_MODEL, "Sonnet Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_OPUS_MODEL, "Opus Model Override", FieldType::String).optional(),
    ProviderField::field(
        ENV_EFFORT_LEVEL,
        "Effort Level",
        FieldType::Choice {
            options: EFFORT_LEVEL_OPTIONS,
        },
    )
    .optional(),
];

static NVIDIA_NIM_FIELDS: &[ProviderField] = &[
    ProviderField::field(ENV_BASE_URL, "API Base URL", FieldType::Url)
        .required()
        .default("https://integrate.api.nvidia.com/v1"),
    ProviderField::field(ENV_AUTH_TOKEN, "API Key", FieldType::Secret).required(),
    ProviderField::field(ENV_HAIKU_MODEL, "Haiku Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_SONNET_MODEL, "Sonnet Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_OPUS_MODEL, "Opus Model Override", FieldType::String).optional(),
    ProviderField::field(
        ENV_EFFORT_LEVEL,
        "Effort Level",
        FieldType::Choice {
            options: EFFORT_LEVEL_OPTIONS,
        },
    )
    .optional(),
];

static OLLAMA_FIELDS: &[ProviderField] = &[
    ProviderField::field(ENV_BASE_URL, "Ollama URL", FieldType::Url)
        .required()
        .default("http://localhost:11434"),
    ProviderField::field(ENV_AUTH_TOKEN, "Auth Token", FieldType::Secret).optional(),
    ProviderField::field(ENV_HAIKU_MODEL, "Haiku Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_SONNET_MODEL, "Sonnet Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_OPUS_MODEL, "Opus Model Override", FieldType::String).optional(),
    ProviderField::field(
        ENV_EFFORT_LEVEL,
        "Effort Level",
        FieldType::Choice {
            options: EFFORT_LEVEL_OPTIONS,
        },
    )
    .optional(),
];

static LMSTUDIO_FIELDS: &[ProviderField] = &[
    ProviderField::field(ENV_BASE_URL, "LM Studio URL", FieldType::Url)
        .required()
        .default("http://localhost:1234"),
    ProviderField::field(ENV_AUTH_TOKEN, "Auth Token", FieldType::Secret)
        .required()
        .default("lm-studio"),
    ProviderField::field(ENV_HAIKU_MODEL, "Haiku Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_SONNET_MODEL, "Sonnet Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_OPUS_MODEL, "Opus Model Override", FieldType::String).optional(),
    ProviderField::field(
        ENV_EFFORT_LEVEL,
        "Effort Level",
        FieldType::Choice {
            options: EFFORT_LEVEL_OPTIONS,
        },
    )
    .optional(),
];

static VLLM_FIELDS: &[ProviderField] = &[
    ProviderField::field(ENV_BASE_URL, "vLLM URL", FieldType::Url)
        .required()
        .default("http://localhost:8000"),
    ProviderField::field(ENV_AUTH_TOKEN, "Auth Token", FieldType::Secret).optional(),
    ProviderField::field(ENV_HAIKU_MODEL, "Haiku Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_SONNET_MODEL, "Sonnet Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_OPUS_MODEL, "Opus Model Override", FieldType::String).optional(),
    ProviderField::field(
        ENV_EFFORT_LEVEL,
        "Effort Level",
        FieldType::Choice {
            options: EFFORT_LEVEL_OPTIONS,
        },
    )
    .optional(),
];

static LITELLM_FIELDS: &[ProviderField] = &[
    ProviderField::field(ENV_BASE_URL, "LiteLLM URL", FieldType::Url)
        .required()
        .default("http://localhost:4000"),
    ProviderField::field(ENV_AUTH_TOKEN, "API Key", FieldType::Secret).optional(),
    ProviderField::field(ENV_HAIKU_MODEL, "Haiku Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_SONNET_MODEL, "Sonnet Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_OPUS_MODEL, "Opus Model Override", FieldType::String).optional(),
    ProviderField::field(
        ENV_EFFORT_LEVEL,
        "Effort Level",
        FieldType::Choice {
            options: EFFORT_LEVEL_OPTIONS,
        },
    )
    .optional(),
];

static CLOUDFLARE_FIELDS: &[ProviderField] = &[
    ProviderField::field(ENV_BASE_URL, "Gateway URL", FieldType::Url).required(),
    ProviderField::field(ENV_AUTH_TOKEN, "API Key", FieldType::Secret).required(),
    ProviderField::field(ENV_HAIKU_MODEL, "Haiku Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_SONNET_MODEL, "Sonnet Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_OPUS_MODEL, "Opus Model Override", FieldType::String).optional(),
    ProviderField::field(
        ENV_EFFORT_LEVEL,
        "Effort Level",
        FieldType::Choice {
            options: EFFORT_LEVEL_OPTIONS,
        },
    )
    .optional(),
];

static VERCEL_FIELDS: &[ProviderField] = &[
    ProviderField::field(ENV_BASE_URL, "Gateway URL", FieldType::Url).required(),
    ProviderField::field(ENV_AUTH_TOKEN, "API Key", FieldType::Secret).required(),
    ProviderField::field(ENV_HAIKU_MODEL, "Haiku Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_SONNET_MODEL, "Sonnet Model Override", FieldType::String).optional(),
    ProviderField::field(ENV_OPUS_MODEL, "Opus Model Override", FieldType::String).optional(),
    ProviderField::field(
        ENV_EFFORT_LEVEL,
        "Effort Level",
        FieldType::Choice {
            options: EFFORT_LEVEL_OPTIONS,
        },
    )
    .optional(),
];

// ---- Registry ------------------------------------------------------------

pub static PROVIDERS: &[ProviderDef] = &[
    ProviderDef {
        id: "anthropic-compatible",
        name: "Any Anthropic or OpenAI Compatible",
        fields: ANTHROPIC_COMPATIBLE_FIELDS,
        supports_statusline: false,
        groups: None,
    },
    ProviderDef {
        id: "zai",
        name: "z.ai (GLM)",
        fields: ZAI_FIELDS,
        supports_statusline: false,
        groups: None,
    },
    ProviderDef {
        id: "openrouter",
        name: "OpenRouter",
        fields: OPENROUTER_FIELDS,
        supports_statusline: true,
        groups: Some(OPENROUTER_GROUPS),
    },
    ProviderDef {
        id: "deepseek",
        name: "DeepSeek",
        fields: DEEPSEEK_FIELDS,
        supports_statusline: true,
        groups: None,
    },
    ProviderDef {
        id: "minimax",
        name: "MiniMax",
        fields: MINIMAX_FIELDS,
        supports_statusline: false,
        groups: None,
    },
    ProviderDef {
        id: "glm",
        name: "GLM (ZhipuAI)",
        fields: GLM_FIELDS,
        supports_statusline: false,
        groups: None,
    },
    ProviderDef {
        id: "moonshot",
        name: "Moonshot",
        fields: MOONSHOT_FIELDS,
        supports_statusline: false,
        groups: None,
    },
    ProviderDef {
        id: "qwen",
        name: "Qwen (Alibaba)",
        fields: QWEN_FIELDS,
        supports_statusline: false,
        groups: None,
    },
    ProviderDef {
        id: "fireworks",
        name: "Fireworks AI",
        fields: FIREWORKS_FIELDS,
        supports_statusline: false,
        groups: None,
    },
    ProviderDef {
        id: "volcengine",
        name: "Volcengine",
        fields: VOLCENGINE_FIELDS,
        supports_statusline: false,
        groups: None,
    },
    ProviderDef {
        id: "nvidia-nim",
        name: "NVIDIA NIM",
        fields: NVIDIA_NIM_FIELDS,
        supports_statusline: false,
        groups: None,
    },
    ProviderDef {
        id: "ollama",
        name: "Ollama (local)",
        fields: OLLAMA_FIELDS,
        supports_statusline: false,
        groups: None,
    },
    ProviderDef {
        id: "lmstudio",
        name: "LM Studio",
        fields: LMSTUDIO_FIELDS,
        supports_statusline: false,
        groups: None,
    },
    ProviderDef {
        id: "vllm",
        name: "vLLM",
        fields: VLLM_FIELDS,
        supports_statusline: false,
        groups: None,
    },
    ProviderDef {
        id: "litellm",
        name: "LiteLLM",
        fields: LITELLM_FIELDS,
        supports_statusline: false,
        groups: None,
    },
    ProviderDef {
        id: "cloudflare",
        name: "Cloudflare AI Gateway",
        fields: CLOUDFLARE_FIELDS,
        supports_statusline: false,
        groups: None,
    },
    ProviderDef {
        id: "vercel",
        name: "Vercel AI Gateway",
        fields: VERCEL_FIELDS,
        supports_statusline: false,
        groups: None,
    },
    ProviderDef {
        id: PROVIDER_LLAMACPP_SINGLE_MODEL,
        name: "llama.cpp (single model, auto-detect)",
        fields: llamacpp::LLAMACPP_FIELDS,
        supports_statusline: false,
        groups: None,
    },
];

pub fn get_provider(id: &str) -> Option<&'static ProviderDef> {
    PROVIDERS.iter().find(|p| p.id == id)
}
