#pragma once
#ifndef AST_H
#define AST_H

#include <variant>
#include <optional>
#include <vector>
#include <string>

#include "enums.h"
#include "util.h"
#include "context.h"
#include "lex.h"

template<class T>
class DefinitionCore;

template<class T>
class AstNode {
  public:
  std::string name;
  Location loc;
  T info;

  AstNode(std::string name, Location loc): name{name}, loc{loc} {}
};

template<class T>
class ValueCore : public AstNode<T> {
  public:
  // TODO: support non symbol/operator values.
  // e.g. numbers, strings, arrays, sets.
  std::vector<DefinitionCore<T>> args;

  ValueCore() = delete;
  ValueCore(std::string name, Location loc, std::vector<DefinitionCore<T>> args):
    AstNode<T>(name, loc),
    args{args} {}

  bool operator ==(const ValueCore<T>& other) const {
    if (this->name != other.name) return false;
    if (args.size() != other.args.size()) return false;
    auto it = args.begin();
    auto o_it = other.args.begin();
    while (it != args.end()) {
      if(*it != *o_it) return false;
      it++;
      o_it++;
    }
    return true;
  }
  bool operator !=(const ValueCore<T>& other) const {
    return !(*this == other);
  }

};

template<class T>
class DefinitionCore : public ValueCore<T> {
  public:
  std::optional<ValueCore<T>> value;
  DefinitionCore() = delete;
  DefinitionCore(const std::string name, Location loc, std::vector<DefinitionCore<T>>args, std::optional<ValueCore<T>> value):
    ValueCore<T>(name, loc, args),
    value{value} {}
};

template<class T>
class ModuleCore : public AstNode<T>{
  public:
  std::vector<DefinitionCore<T>> definitions;
  ModuleCore() = delete;
  ModuleCore(std::string name, Location loc, std::vector<DefinitionCore<T>>definitions): AstNode<T>(name, loc), definitions{definitions} {}
};

struct Empty { };

using Value = ValueCore<Empty>;
using Definition = DefinitionCore<Empty>;
using Module = ModuleCore<Empty>;

Tree<Token> ast(Tokens& toks, Context &ctx);

#endif // #ifndef AST_H