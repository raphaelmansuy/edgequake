import { marked } from 'marked';

const testContent = "**MegaRAG** is a variant of **RAG** and has been developed by **National Taiwan University**.";
const badContent = "**MegaRAG**is a variant of**RAG**and has been developed by**National Taiwan University**.";

console.log("=== GOOD CONTENT (with spaces) ===");
console.log("Input:", testContent);
const tokens = marked.lexer(testContent);
console.log("Paragraph tokens:", JSON.stringify(tokens[0].tokens, null, 2));

console.log("\n=== BAD CONTENT (no spaces) ===");
console.log("Input:", badContent);
const badTokens = marked.lexer(badContent);
console.log("Paragraph tokens:", JSON.stringify(badTokens[0].tokens, null, 2));
