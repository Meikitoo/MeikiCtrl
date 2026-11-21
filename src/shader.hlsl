// MeikiCtrl — éclairage + coque à éclairs animés (cyan Meikitoow).

cbuffer Frame : register(b0)
{
    float4x4 World;
    float4x4 ViewProj;
    float4x4 NormalMat;
    float4x4 RootInv;
    float4 BaseColor;
    float4 Emissive;
    float4 Params;  // x = éclairs, y = temps, z = lueur, w = énergie
    float4 CamPos;
    float4 Accent;  // rgb = couleur des éclairs, w = opacité de l'overlay
};

#define CYAN (Accent.rgb)

struct VSIn
{
    float3 pos : POSITION;
    float3 nrm : NORMAL;
};

struct VSOut
{
    float4 pos  : SV_POSITION;
    float3 wpos : TEXCOORD0;
    float3 nrm  : TEXCOORD1;
};

VSOut VSMain(VSIn i)
{
    VSOut o;
    float4 w = mul(World, float4(i.pos, 1.0));
    o.pos = mul(ViewProj, w);
    o.wpos = w.xyz;
    o.nrm = mul((float3x3)NormalMat, i.nrm);
    return o;
}

// --- Bruit procédural ---
float hash3(float3 p)
{
    p = frac(p * 0.3183099 + 0.1);
    p *= 17.0;
    return frac(p.x * p.y * p.z * (p.x + p.y + p.z));
}

float noise3(float3 x)
{
    float3 i = floor(x);
    float3 f = frac(x);
    f = f * f * (3.0 - 2.0 * f);
    return lerp(
        lerp(lerp(hash3(i + float3(0, 0, 0)), hash3(i + float3(1, 0, 0)), f.x),
             lerp(hash3(i + float3(0, 1, 0)), hash3(i + float3(1, 1, 0)), f.x), f.y),
        lerp(lerp(hash3(i + float3(0, 0, 1)), hash3(i + float3(1, 0, 1)), f.x),
             lerp(hash3(i + float3(0, 1, 1)), hash3(i + float3(1, 1, 1)), f.x), f.y),
        f.z);
}

// Bruit « en crêtes » : donne des filaments fins, comme des éclairs.
float ridged(float3 p)
{
    float s = 0.0;
    float a = 0.5;
    [unroll] for (int k = 0; k < 4; k++)
    {
        float n = 1.0 - abs(noise3(p) * 2.0 - 1.0);
        s += n * n * a;
        p *= 2.03;
        a *= 0.5;
    }
    return s;
}

float4 PSMain(VSOut i) : SV_TARGET
{
    float3 N = normalize(i.nrm);
    float3 V = normalize(CamPos.xyz - i.wpos);
    if (dot(N, V) < 0.0) N = -N;
    float3 L = normalize(float3(-0.35, 1.0, -0.55));

    float3 albedo = BaseColor.rgb;
    float3 glowc = float3(0.0, 0.0, 0.0);

    if (Params.x > 0.5)
    {
        float t = Params.y;
        // Position dans le repère de la manette : les éclairs restent collés à la coque.
        float3 p = mul(RootInv, float4(i.wpos, 1.0)).xyz;
        float swirl = noise3(p * 1.3 + float3(t * 0.07, 0.0, t * 0.05));
        // Fond presque noir, à peine bleuté : c'est le contraste qui fait briller les éclairs.
        albedo = lerp(float3(0.006, 0.007, 0.01), float3(0.0, 0.025, 0.04), swirl * swirl);
        float r = ridged(p * 2.1 + float3(swirl * 0.9, t * 0.12, t * 0.18));
        float veins = smoothstep(0.68, 0.9, r);
        float core = pow(smoothstep(0.8, 0.95, r), 2.0);  // cœur blanc des éclairs
        float flick = 0.8 + 0.2 * sin(t * 7.0 + p.x * 3.0) * sin(t * 3.1);
        float power = (0.5 + 2.0 * Params.w) * flick;
        glowc = (CYAN * veins + float3(0.7, 1.0, 1.0) * core * 0.6) * power;
    }

    float diff = saturate(dot(N, L)) * 0.8 + 0.2;
    // Reflet net de plastique laqué, sans voile.
    float spec = pow(saturate(dot(N, normalize(L + V))), 120.0) * 0.35;
    float rim = pow(1.0 - saturate(dot(N, V)), 4.0);

    // Lueur des boutons : mélange avec la couleur de la pièce pour garder ses reliefs.
    float3 lit = albedo * diff + spec;
    float3 glowBtn = Emissive.rgb * Params.z;
    float3 c = lerp(lit, lit * 0.3 + glowBtn * (0.7 + 0.6 * diff), saturate(Params.z))
             + rim * CYAN * 0.06 + glowc;
    c = c / (1.0 + c * 0.35);          // tonemap doux
    c = pow(saturate(c), 1.0 / 2.2);   // gamma
    return float4(c * Accent.w, Accent.w);  // alpha prémultiplié : opacité réglable
}
