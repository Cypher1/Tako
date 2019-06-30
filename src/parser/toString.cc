#include <iostream>
#include <string>
#include <sstream>
#include <algorithm>

#include "../util/util.h"

#include "ast.h"
#include "toString.h"

int width = 80;
int height = 80;

void indent(std::stringstream& o, int depth) {
  for(int i=0; i<depth; i++) {
    o << " ";
  }
}

std::string getString(const Location& loc, const std::string& contents) {
  return contents.substr(loc.start, loc.length);
}

std::string toString(const Location& loc, const std::string& contents, const std::string& filename, int depth) {
  size_t line = 1+std::count(contents.begin(), contents.begin()+loc.start, '\n');
  size_t col = loc.start - contents.rfind("\n", loc.start);
  std::stringstream o;
  indent(o, depth);
  o << " line " << line;
  o << " column " << col;
  return o.str();
}

std::string toString(const Value& val, const std::string& contents, const std::string& filename, int depth) {
  std::stringstream o;
  indent(o, depth);
  o << val.name;
  if (!val.args.empty()) {
    o<< "(" << toString(val.args, contents, filename, 0, ", ") << ")";
  }
  return o.str();
}

std::string toString(const Definition& val, const std::string& contents, const std::string& filename, int depth) {
  std::stringstream o;
  indent(o, depth);
  o << val.name;
  if (!val.args.empty()) {
    o<< "(" << toString(val.args, contents, filename, 0) << ")";
  }
  if (val.value) {
    o << " = " << toString(*val.value, contents, filename, 0);
  }
  return o.str();
}

std::string toString(const FuncArg& arg, const std::string& contents, const std::string& filename, int depth) {
  std::stringstream o;
  indent(o, depth);
  o << "[" << arg.ord << "]" << toString(Definition(arg), contents, filename, 0);
  return o.str();
}

std::string toString(const Token& tok, const std::string& contents, const std::string& filename, int depth) {
  std::stringstream o;
  indent(o, depth);
  if (tok.type == +TokenType::WhiteSpace) {
    o << "'";
  }
  o << getString(tok.loc, contents);
  if (tok.type == +TokenType::WhiteSpace) {
    o << "'";
  }
  o << " : " << tok.type;
  if(/*show locations*/ false) {
    std::stringstream s;
    s << toString(tok.loc, contents, filename, 0);
    indent(o, width-s.str().length()-o.str().length());
    o << s.str();
  }
  return o.str();
}

std::string toString(const Message& msg, const std::string& contents, const std::string& filename, int depth) {
  std::stringstream o;
  indent(o, depth);
  o << msg.pass << " ";
  o << msg.type << ": ";
  o << msg.msg << " ";
  o << toString(msg.loc, contents, filename, 0);
  return o.str();
}

std::string toString(const Tree<Token>& tree, const std::string& contents, const std::string& filename, int depth) {
  std::stringstream o;
  o << toString(tree.value, contents, filename, depth);
  o << toString(tree.children, contents, filename, depth+2, "\n");
  return o.str();
}

std::string toString(const Module& module, const std::string& contents, const std::string& filename, int depth) {
  std::stringstream o;
  indent(o, depth);
  o << "module " << module.name << " (" << module.definitions.size() << " top level definitions) {\n";
  for(const auto& val : module.definitions) {
    o << toString(val, contents, filename, depth+2) << "\n";
  }
  indent(o, depth);
  o << "}";
  return o.str();
}
