import {
    createConnection,
    TextDocuments,
    ProposedFeatures,
    InitializeParams,
    InitializeResult,
    TextDocumentSyncKind,
    CompletionItem,
    CompletionItemKind
} from 'vscode-languageserver/node';

import { TextDocument } from 'vscode-languageserver-textdocument';

const connection = createConnection(ProposedFeatures.all);
const documents: TextDocuments<TextDocument> = new TextDocuments(TextDocument);

connection.onInitialize((params: InitializeParams) => {
    const result: InitializeResult = {
        capabilities: {
            textDocumentSync: {
                openClose: true,
                change: TextDocumentSyncKind.Incremental,
            },
            completionProvider: {
                resolveProvider: false,
                triggerCharacters: [':', ',', '{'],
            },
        },
    };
    return result;
});

// Validate material on document change
documents.onDidChangeContent((change) => {
    validateMaterial(change.document);
});

async function validateMaterial(textDocument: TextDocument): Promise<void> {
    // Simple validation - check if name and shadingModel exist
    const text = textDocument.getText();
    const diagnostics: any[] = [];

    if (!text.includes('name')) {
        diagnostics.push({
            severity: 1, // Warning
            range: {
                start: { line: 0, character: 0 },
                end: { line: 0, character: 1 },
            },
            message: 'Material should have a "name" property',
            source: 'filament-mat',
        });
    }

    if (!text.includes('shadingModel')) {
        diagnostics.push({
            severity: 1, // Warning
            range: {
                start: { line: 0, character: 0 },
                end: { line: 0, character: 1 },
            },
            message: 'Material should have a "shadingModel" property',
            source: 'filament-mat',
        });
    }

    connection.sendDiagnostics({ uri: textDocument.uri, diagnostics });
}

// Completion provider
connection.onCompletion(() => {
    const completions: CompletionItem[] = [
        // Material properties
        { label: 'name', kind: CompletionItemKind.Property, documentation: 'Material name identifier' },
        { label: 'shadingModel', kind: CompletionItemKind.Property, documentation: 'Shading model (lit/unlit/subsurface/etc)' },
        { label: 'requires', kind: CompletionItemKind.Property, documentation: 'Required vertex attributes' },
        { label: 'parameters', kind: CompletionItemKind.Property, documentation: 'Material parameters list' },
        { label: 'constants', kind: CompletionItemKind.Property, documentation: 'Material constants' },
        { label: 'culling', kind: CompletionItemKind.Property, documentation: 'Face culling (front/back/none)' },
        { label: 'blending', kind: CompletionItemKind.Property, documentation: 'Blending mode' },
        { label: 'vertexDomain', kind: CompletionItemKind.Property, documentation: 'Vertex domain (object/world/view/device)' },
        { label: 'doubleSided', kind: CompletionItemKind.Property, documentation: 'Whether material is two-sided' },
        { label: 'colorWrite', kind: CompletionItemKind.Property, documentation: 'Enable color write' },
        { label: 'depthWrite', kind: CompletionItemKind.Property, documentation: 'Enable depth write' },
        
        // Shading model values
        { label: 'lit', kind: CompletionItemKind.Enum, documentation: 'Standard PBR shading' },
        { label: 'unlit', kind: CompletionItemKind.Enum, documentation: 'Unlit shading, no lighting' },
        { label: 'subsurface', kind: CompletionItemKind.Enum, documentation: 'Subsurface scattering' },
        { label: 'cloth', kind: CompletionItemKind.Enum, documentation: 'Cloth shading' },
        { label: 'specularGlossiness', kind: CompletionItemKind.Enum, documentation: 'Specular-glossiness workflow' },
        
        // Blending values
        { label: 'opaque', kind: CompletionItemKind.Enum, documentation: 'Opaque blending' },
        { label: 'transparent', kind: CompletionItemKind.Enum, documentation: 'Alpha blending' },
        { label: 'fade', kind: CompletionItemKind.Enum, documentation: 'Fade transparency' },
        { label: 'masked', kind: CompletionItemKind.Enum, documentation: 'Alpha mask (binary)' },
        { label: 'add', kind: CompletionItemKind.Enum, documentation: 'Additive blending' },
        { label: 'custom', kind: CompletionItemKind.Enum, documentation: 'Custom blending' },
        
        // Parameter types
        { label: 'bool', kind: CompletionItemKind.TypeParameter, documentation: 'Boolean value' },
        { label: 'bool2', kind: CompletionItemKind.TypeParameter, documentation: '2-component boolean vector' },
        { label: 'bool3', kind: CompletionItemKind.TypeParameter, documentation: '3-component boolean vector' },
        { label: 'bool4', kind: CompletionItemKind.TypeParameter, documentation: '4-component boolean vector' },
        { label: 'int', kind: CompletionItemKind.TypeParameter, documentation: 'Integer value' },
        { label: 'int2', kind: CompletionItemKind.TypeParameter, documentation: '2-component integer vector' },
        { label: 'int3', kind: CompletionItemKind.TypeParameter, documentation: '3-component integer vector' },
        { label: 'int4', kind: CompletionItemKind.TypeParameter, documentation: '4-component integer vector' },
        { label: 'uint', kind: CompletionItemKind.TypeParameter, documentation: 'Unsigned integer' },
        { label: 'uint2', kind: CompletionItemKind.TypeParameter, documentation: '2-component uint vector' },
        { label: 'uint3', kind: CompletionItemKind.TypeParameter, documentation: '3-component uint vector' },
        { label: 'uint4', kind: CompletionItemKind.TypeParameter, documentation: '4-component uint vector' },
        { label: 'float', kind: CompletionItemKind.TypeParameter, documentation: 'Floating point value' },
        { label: 'float2', kind: CompletionItemKind.TypeParameter, documentation: '2-component float vector' },
        { label: 'float3', kind: CompletionItemKind.TypeParameter, documentation: '3-component float vector' },
        { label: 'float4', kind: CompletionItemKind.TypeParameter, documentation: '4-component float vector' },
        { label: 'mat3', kind: CompletionItemKind.TypeParameter, documentation: '3x3 matrix' },
        { label: 'mat4', kind: CompletionItemKind.TypeParameter, documentation: '4x4 matrix' },
        { label: 'sampler2d', kind: CompletionItemKind.TypeParameter, documentation: '2D texture sampler' },
        { label: 'sampler3d', kind: CompletionItemKind.TypeParameter, documentation: '3D texture sampler' },
        { label: 'samplerCubemap', kind: CompletionItemKind.TypeParameter, documentation: 'Cube map sampler' },
        { label: 'samplerExternal', kind: CompletionItemKind.TypeParameter, documentation: 'External image sampler' },
        
        // Requires values
        { label: 'position', kind: CompletionItemKind.Enum, documentation: 'Vertex position' },
        { label: 'normal', kind: CompletionItemKind.Enum, documentation: 'Vertex normal' },
        { label: 'uv0', kind: CompletionItemKind.Enum, documentation: 'UV coordinate set 0' },
        { label: 'uv1', kind: CompletionItemKind.Enum, documentation: 'UV coordinate set 1' },
        { label: 'color', kind: CompletionItemKind.Enum, documentation: 'Vertex color' },
        { label: 'tangents', kind: CompletionItemKind.Enum, documentation: 'Tangent and bitangent' },
        { label: 'custom0', kind: CompletionItemKind.Enum, documentation: 'Custom attribute 0' },
        { label: 'custom1', kind: CompletionItemKind.Enum, documentation: 'Custom attribute 1' },
        { label: 'custom2', kind: CompletionItemKind.Enum, documentation: 'Custom attribute 2' },
        { label: 'custom3', kind: CompletionItemKind.Enum, documentation: 'Custom attribute 3' },
        { label: 'custom4', kind: CompletionItemKind.Enum, documentation: 'Custom attribute 4' },
        { label: 'boneIndices', kind: CompletionItemKind.Enum, documentation: 'Bone indices for skinning' },
        { label: 'boneWeights', kind: CompletionItemKind.Enum, documentation: 'Bone weights for skinning' },
    ];

    return completions;
});

documents.listen(connection);
connection.listen();
