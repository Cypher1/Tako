#include <functional>
#include <iostream>
#include <optional>
#include <ostream>
#include <string>
#include <variant>

#include "ast.h"
#include "eval.h"
#include "parser.h"
#include "show.h"

std::ostream& operator<<(std::ostream& o, const PrimError e) {
  o << e.msg << "\n";
  return o;
}

std::string repeat(int n, std::string rep) {
  std::string o = rep;
  while (o.length() < rep.length() * n) {
    o += o;
  }
  return o.substr(0, rep.length() * n);
}

std::string repeatR(std::string rep, int n) {
  return repeat(n, rep);
}

template<typename T>
Prim mins(T x, T y) { return x - y; }

template<typename T>
Prim add(T x, T y) { return x + y; }

template<typename T>
Prim mult(T x, T y) { return x * y; }

TryPrim require(const Pred req, const TryPrim cont) {
  return [=]() -> OptPrim {
    if (req()) {
      return cont();
    }
    return std::nullopt;
  };
}

TryPrim tryEach(const TryPrims fs, const PrimError msg) {
  return [=]() -> OptPrim {
    for (const auto& f : fs) {
      const OptPrim v = f();
      if (v) {
        return v;
      }
    }
    return Prim(msg);
  };
}

template<typename T, typename U>
TryPrim operator2(const std::string name, const Prims vals, const std::function<Prim(T, U)> f) {
    if (vals.size() != 2) {
      return [name](){ return PrimError("Expected two arguments at !!! " + name);};
    }
  return [vals, f]() -> OptPrim {
    auto x = vals[0];
    if (!std::holds_alternative<T>(x)) {
      return std::nullopt;
    }
    auto y = vals[1];
    if (!std::holds_alternative<U>(y)) {
      return std::nullopt;
    }
    return f(std::get<T>(x), std::get<U>(y));
  };
}

Prim eval(Value val, parser::ParserContext& p_ctx) {
  // TODO: Eval
  if (val.node_type == AstNodeType::Text) {
    // Get the text
    return val.name.substr(1, val.name.length() - 2);
  }

  if (val.node_type == AstNodeType::Numeric) {
    // Get the number
    std::cerr << "v:" << std::stoi(val.name, nullptr, 10) << "\n"; // Assume base 10
    return std::stoi(val.name, nullptr, 10); // Assume base 10
  }
  if (val.node_type == AstNodeType::Symbol) {
    // Look up the symbol
    const std::vector<Definition> args = val.args;

    std::vector<Prim> values;
    for (const auto &arg : args) {
      if (arg.value) {
        values.push_back(eval(*arg.value, p_ctx));
      } else {
        return PrimError("Missing value for arg in !!! " + val.name);
      }
    }

    const TryPrim adders =
      require(
          [val]{return val.name == "+";},
          tryEach({
            operator2<int, int>("+", values, add<int>),
            operator2<std::string, std::string>("+", values, add<std::string>)
            }, "Unexpected types at (+) !!! " + val.name)
          );

    const TryPrim subs =
      require(
          [val]{return val.name == "-";},
          tryEach({
            operator2<int, int>("-", values, mins<int>),
            }, "Unexpected types at (-) !!! " + val.name)
          );

    const TryPrim mults =
      require(
          [val]{return val.name == "*";},
          tryEach({
            operator2<int, int>("*", values, mult<int>),
            operator2<std::string, int>("*", values, repeatR),
            operator2<int, std::string>("*", values, repeat)
            }, "Unexpected types at (*) !!! " + val.name)
          );

    const OptPrim v = tryEach({adders, subs, mults}, "Unknown symbol !!! " + val.name)();
    if (v) {
      return *v;
    }
  }
  return PrimError("OH NO!!! " + val.name);
}

Prim eval(Module mod, parser::ParserContext& p_ctx) {
  auto o_def = p_ctx.getTable().lookup({}, {"main"});
  if (o_def) {
    auto def = *o_def;
    if (def.value) {
      auto val = *def.value;
      return eval(val, p_ctx);
    } else {
      return PrimError("main has no set value");
    }
  }
  return PrimError("Module has no main");
}