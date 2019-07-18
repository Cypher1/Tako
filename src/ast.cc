#include <functional>
#include <iostream>
#include <map>
#include <set>
#include <stdexcept> // TODO: Remove use of exceptions, instead use messages and fallback.
#include <string>
#include <vector>

#include "context.h"

#include "ast.h"
#include "ast_internal.h"
#include "lex.h"

const std::map<TokenType, TokenType> brackets = {
    {TokenType::OpenParen, TokenType::CloseParen},
    {TokenType::OpenBrace, TokenType::CloseBrace},
    {TokenType::OpenBracket, TokenType::CloseBracket},
    {TokenType::SingleQuote, TokenType::SingleQuote},
    {TokenType::DoubleQuote, TokenType::DoubleQuote},
    {TokenType::BackQuote, TokenType::BackQuote},
};

const std::map<TokenType, TokenType> close_brackets = []() {
  std::map<TokenType, TokenType> map;
  for (auto kv : brackets) {
    if (brackets.find(kv.second) == brackets.end()) {
      map.emplace(kv.second, kv.first);
    }
  }
  return map;
}();

constexpr bool isQuote(const TokenType &type) {
  return type == +TokenType::SingleQuote || type == +TokenType::DoubleQuote ||
         type == +TokenType::BackQuote;
}

const std::map<std::string, unsigned int> symbol_binding = {};
const leftBindingPowerType symbolBind = [](const Token &tok,
                                           const ParserContext &ctx) {
  auto p_it = symbol_binding.find(ctx.context.getStringAt(tok.loc));
  if (p_it == symbol_binding.end()) {
    return 0u;
  }
  return p_it->second;
};

const std::map<std::string, unsigned int> infix_binding = {
    {"-|", 20}, {"|-", 30},  {"=", 40},   {"<", 60},  {"<=", 60}, {">", 60},
    {">=", 60}, {"<>", 60},  {"!=", 60},  {"==", 60}, {"|", 70},  {"^", 80},
    {"&", 90},  {"<<", 100}, {">>", 100}, {"+", 110}, {"-", 110}, {"*", 120},
    {"/", 120}, {"//", 120}, {"%", 120},  {":", 130}, {".", 140}, {"[", 150},
    {"(", 150}, {"{", 150}};

const std::map<std::string, unsigned int> prefix_binding = {
    {"-", 130}, {"+", 130}, {"~", 130}, {"!", 130}};

const auto operatorBind = [](const Token &tok, ParserContext &ctx) { // Lbp
  std::string t = ctx.context.getStringAt(tok.loc);
  auto p_it = infix_binding.find(t);
  if (p_it == infix_binding.end()) {
    std::string start = std::to_string(tok.loc.start);
    std::string len = std::to_string(tok.loc.length);
    throw std::runtime_error(
      std::string() + "Expected an infix operator but found("+start+","+len+") '" + t + "'"
    );
  }
  return p_it->second;
};

Tree<Token> prefixOp(const Token &tok, ParserContext &ctx) {
  auto root = Tree<Token>(tok);
  std::string t = ctx.context.getStringAt(tok.loc);
  auto p_it = prefix_binding.find(t);
  if (p_it == prefix_binding.end()) {
    throw std::runtime_error(
      std::string() + "Expected a prefix operator but found '" + t + "'"
    );
  }
  auto right = ast::parseValue(ctx, p_it->second);
  root.children.push_back(right);
  return root;
};

Tree<Token> infixOp(Tree<Token> left, const Token &tok, ParserContext &ctx) {
  auto root = Tree<Token>(tok, {left});
  // Led
  auto p_it = infix_binding.find(ctx.context.getStringAt(tok.loc));
  if (p_it == infix_binding.end()) {
    // TODO Defaulting is bad...
  };
  auto right = ast::parseValue(ctx, p_it->second);
  root.children.push_back(right);
  return root;
};

Tree<Token> symbol(const Token &tok, ParserContext &ctx) { // Led
  return Tree<Token>(tok);
};

Tree<Token> ignoreInit(const Token &, ParserContext &) {
  return {errorToken, {}};
};
Tree<Token> ignore(const Tree<Token> left, const Token &, ParserContext &) {
  return left;
};

Tree<Token> bracket(const Token &tok, ParserContext &ctx) { // Nud
  std::vector<Tree<Token>> inner;
  const auto close_it = brackets.find(tok.type);
  if (close_it == brackets.end()) {
    throw std::runtime_error(std::string() + "Unknown bracket type " +
                             tok.type._to_string());
  }
  const auto closeTT = close_it->second;
  while (ctx.hasToken && (ctx.getCurr().type != closeTT)) {
    auto exp = ast::parseValue(ctx);
    inner.push_back(exp);
  }
  ctx.expect(closeTT);

  return {tok, inner};
};

Tree<Token> funcArgs(Tree<Token> left, const Token &tok,
                     ParserContext &ctx) { // Led
  std::vector<Tree<Token>> inner;
  const auto close_it = brackets.find(tok.type);
  if (close_it == brackets.end()) {
    throw std::runtime_error(std::string() + "Unknown bracket type " +
                             tok.type._to_string());
  }
  const auto closeTT = close_it->second;
  while (ctx.hasToken && (ctx.getCurr().type != closeTT)) {
    auto exp = ast::parseValue(ctx);
    inner.push_back(exp);
  }
  ctx.expect(closeTT);

  left.children = inner;
  return left; // This is a function call
};

std::map<TokenType, SymbolTableEntry> symbolTable = {
    {TokenType::Comma, {operatorBind, infixOp}},
    {TokenType::Operator, {operatorBind, prefixOp, infixOp}},
    {TokenType::PreCond, {operatorBind, infixOp}},
    {TokenType::PostCond, {operatorBind, infixOp}},
    {TokenType::SemiColon, {symbolBind, ignore}},
    {TokenType::Symbol, {symbolBind, symbol}},
    {TokenType::OpenParen, {operatorBind, bracket, funcArgs}},
    {TokenType::CloseParen,
     {symbolBind, ignoreInit, ignore}}, // TODO: Warning / error on unmatched.
    {TokenType::OpenBrace, {operatorBind, bracket}},
    {TokenType::CloseBrace,
     {symbolBind, ignoreInit, ignore}}, // TODO: Warning / error on unmatched.
    {TokenType::OpenBracket, {operatorBind, bracket}},
    {TokenType::CloseBracket,
     {symbolBind, ignoreInit, ignore}}, // TODO: Warning / error on unmatched.
    {TokenType::DoubleQuote,
     {operatorBind, bracket}}, // TODO: Warning / error on unmatched.
    {TokenType::SingleQuote,
     {operatorBind, bracket}}, // TODO: Warning / error on unmatched.
    {TokenType::BackQuote,
     {operatorBind, bracket}}, // TODO: Warning / error on unmatched.
    {TokenType::NumberLiteral, {symbolBind, symbol}},
    {TokenType::Dot, {symbolBind, symbol}},
    {TokenType::Error, {symbolBind, symbol}},
};

bool ParserContext::next() {
  if (hasToken) {
    // std::cout << "> " << context.getStringAt(getCurr().loc) << "\n"; // For debugging.
    if (toks != end) {
      toks++;
    }
    if (toks != end) {
      if (toks->type == +TokenType::WhiteSpace ||
          toks->type == +TokenType::Comma ||
          toks->type == +TokenType::SemiColon) {
        return next(); // TODO instring...
      }
      return true;
    }
  }
  hasToken = false;
  return false;
}

bool ParserContext::expect(const TokenType &expected) {
  if (getCurr().type != expected) {
    msg(MessageType::Error,
        std::string() + "Expected a " + expected._to_string() + " but found " +
            getCurr().type._to_string() + " '" + getCurrString() + "'");
  }
  return next();
}

void ParserContext::msg(MessageType level, std::string msg_txt) {
  // TODO make this print as EOF
  Location loc = eofToken.loc;
  if (hasToken) {
    loc = toks->loc;
  }
  context.msg(loc, level, msg_txt);
}

const Token &ParserContext::getCurr() const {
  if (toks == end) {
    return eofToken;
  }
  return *toks;
}

std::string ParserContext::getCurrString() const {
  return context.getStringAt(getCurr().loc);
}

const SymbolTableEntry ParserContext::entry() {
  auto t = getCurr();
  const auto symbol_it = symbolTable.find(t.type);
  if (symbol_it == symbolTable.end()) {
    throw std::runtime_error(std::string() + t.type._to_string() + +" '" +
                             getCurrString() + "' not found in symbol table");
  }
  return symbol_it->second;
}

Tree<Token> ast::parseDefinition(ParserContext &ctx, unsigned int rbp) {
  // TODO check this is a value (merge with parse pass?)
  return ast::parseValue(ctx, rbp);
}

Tree<Token> ast::parseValue(ParserContext &ctx, unsigned int rbp) {
  unsigned int binding = 0;
  Token t = ctx.getCurr();
  const auto t_entry = ctx.entry();
  ctx.next();
  Tree<Token> left = t_entry.nud(t, ctx);
  binding = ctx.entry().binding(ctx.getCurr(), ctx);
  while (rbp < binding && ctx.hasToken) {
    t = ctx.getCurr();
    const auto t_entry = ctx.entry();
    ctx.next();
    left = t_entry.led(left, t, ctx);
    binding = ctx.entry().binding(ctx.getCurr(), ctx);
  }
  return left;
}

Tree<Token> ast::parseModule(ParserContext &ctx, unsigned int rbp) {
  Forest<Token> definitions;
  while (ctx.hasToken) {
    definitions.push_back(parseDefinition(ctx));
  }
  Token fileToken = {TokenType::Symbol, {0, 0, ctx.context.filename}};
  return Tree<Token>(fileToken, definitions);
}

Tree<Token> ast::ast(Tokens& toks, Context &context, std::function<Tree<Token>(ParserContext &, unsigned int)> func) {
  context.startStep(PassStep::Ast);
  // Add a disposable char to make whitespace dropping easy.
  toks.insert(toks.begin(), errorToken);
  ParserContext ctx(context, toks.cbegin(), toks.cend());
  ctx.next();
  return func(ctx, 0);
}

// TODO convert ast'ifying to a set of functions (with parameterization?)
// ast::parseModule
// ast::parseDefinition
// ast::parseValue
