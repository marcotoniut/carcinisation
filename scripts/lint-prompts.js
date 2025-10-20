#!/usr/bin/env node
/**
 * LLM Prompt Linter
 *
 * Enforces:
 * - Token count limits (policy.min.md ≤1200, system.min.md ≤800)
 * - Banned phrases (try, should, kindly, etc.)
 * - Double-negatives
 * - TODOs/FIXMEs in prompts
 * - RFC 2119 keywords (MUST/MUST NOT/MAY/SHOULD)
 */

const fs = require('fs');
const path = require('path');

// Configuration
const RULES = {
  'policy.min.md': { maxTokens: 1200, maxWords: 900 },
  'system.min.md': { maxTokens: 800, maxWords: 600 },
};

const BANNED_PHRASES = [
  { pattern: /\btry to\b/gi, replacement: 'MUST' },
  { pattern: /\bshould\b/gi, replacement: 'MUST or MAY (RFC 2119)' },
  { pattern: /\bplease\b/gi, replacement: '(remove filler)' },
  { pattern: /\bkindly\b/gi, replacement: '(remove filler)' },
  { pattern: /\bas an AI\b/gi, replacement: '(remove anthropomorphic language)' },
  { pattern: /\bI feel\b/gi, replacement: '(remove anthropomorphic language)' },
  { pattern: /\bwe strive\b/gi, replacement: '(remove filler)' },
  { pattern: /\btry your best\b/gi, replacement: 'MUST' },
];

const DOUBLE_NEGATIVES = [
  /\bnot\s+\w*\s+un\w+/gi,
  /\bnever\s+\w*\s+not\b/gi,
  /\bdon't\s+\w*\s+not\b/gi,
];

// Token estimation (1 word ≈ 1.33 tokens for technical English)
function estimateTokens(text) {
  const words = text.split(/\s+/).filter(w => w.length > 0).length;
  return Math.ceil(words * 1.33);
}

function countWords(text) {
  return text.split(/\s+/).filter(w => w.length > 0).length;
}

// Linting functions
function lintFile(filePath, config) {
  const errors = [];
  const warnings = [];

  if (!fs.existsSync(filePath)) {
    errors.push(`File not found: ${filePath}`);
    return { errors, warnings };
  }

  const content = fs.readFileSync(filePath, 'utf8');
  const words = countWords(content);
  const tokens = estimateTokens(content);

  // Check token limits
  if (config.maxTokens && tokens > config.maxTokens) {
    errors.push(`Token limit exceeded: ${tokens} > ${config.maxTokens} (estimated)`);
  }
  if (config.maxWords && words > config.maxWords) {
    warnings.push(`Word count high: ${words} > ${config.maxWords} (target)`);
  }

  // Check banned phrases
  BANNED_PHRASES.forEach(({ pattern, replacement }) => {
    const matches = content.match(pattern);
    if (matches) {
      errors.push(`Banned phrase found: "${matches[0]}" → use ${replacement}`);
    }
  });

  // Check double-negatives
  DOUBLE_NEGATIVES.forEach(pattern => {
    const matches = content.match(pattern);
    if (matches) {
      warnings.push(`Possible double-negative: "${matches[0]}"`);
    }
  });

  // Check for TODOs/FIXMEs
  const todos = content.match(/\b(TODO|FIXME)\b/gi);
  if (todos) {
    warnings.push(`TODO/FIXME found in prompt (${todos.length} occurrence(s))`);
  }

  // Check RFC 2119 usage
  const hasRFC2119 = /\b(MUST|MUST NOT|MAY|SHOULD|SHOULD NOT)\b/.test(content);
  if (!hasRFC2119) {
    warnings.push('No RFC 2119 keywords found (MUST/MUST NOT/MAY/SHOULD)');
  }

  return { errors, warnings, stats: { words, tokens } };
}

// Main
function main() {
  const mode = process.argv[2];
  const repoRoot = path.resolve(__dirname, '..');

  if (mode === '--audit') {
    console.log('=== LLM Prompt Audit ===\n');

    const files = [
      'docs/llm/policy.min.md',
      'prompts/system.min.md',
    ];

    let totalErrors = 0;
    let totalWarnings = 0;

    files.forEach(file => {
      const filePath = path.join(repoRoot, file);
      const fileName = path.basename(file);
      const config = RULES[fileName] || {};

      console.log(`Checking ${file}...`);
      const { errors, warnings, stats } = lintFile(filePath, config);

      if (stats) {
        console.log(`  Words: ${stats.words}, Tokens (est): ${stats.tokens}`);
      }

      if (errors.length > 0) {
        console.log(`  ❌ Errors (${errors.length}):`);
        errors.forEach(err => console.log(`     - ${err}`));
        totalErrors += errors.length;
      }

      if (warnings.length > 0) {
        console.log(`  ⚠️  Warnings (${warnings.length}):`);
        warnings.forEach(warn => console.log(`     - ${warn}`));
        totalWarnings += warnings.length;
      }

      if (errors.length === 0 && warnings.length === 0) {
        console.log('  ✅ No issues');
      }

      console.log('');
    });

    console.log(`Total: ${totalErrors} error(s), ${totalWarnings} warning(s)`);

    if (totalErrors > 0) {
      process.exit(1);
    }
  } else {
    console.log('Usage: pnpm llm:lint --audit');
    console.log('');
    console.log('Options:');
    console.log('  --audit    Run audit on policy.min.md and system.min.md');
    process.exit(0);
  }
}

if (require.main === module) {
  main();
}

module.exports = { lintFile, estimateTokens, countWords };
