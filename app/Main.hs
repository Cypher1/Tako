module Main where
import Prelude hiding (not, and, or)

-- import Triple
import Expr
import Logic
-- import Util

-- import qualified Data.Map.Lazy as M

main :: IO ()
main = do
  {-putStrLn "hello world"
  print $ var "x"
  print $ Apply (var "f") (var "x")
  let pre = Env [Apply (Apply (var "gr_than") (var "0")) (var "x")]
  let op = Apply (Apply (var "div") (var "y")) (var "x")
  let post = Env []
  let divP = Tri pre (S "div") op post
  let x_eq_2 = (Apply (Apply (var "=") (var "x")) (var "2"))
  let x_eq_3 = (Apply (Apply (var "!=") (var "x")) (var "2"))
  let contra1 = Contradiction x_eq_2 x_eq_3
  let req1 = Unspecified x_eq_2
  let both1 = Many [req1, contra1]
  print divP
  mapM_ print [contra1, req1, both1] -}
  print $ a_or_b
  print $ n_a_or_b
  print $ or [a_or_b, a]
  print $ or [n_a_or_b, na]
  putStrLn ""
  print $ a_or_na
  putStrLn "Test simplification (not, or, and)"
  print $ not a_or_na
  print $ or [a_or_na]
  print $ and [a_or_na]
  putStrLn "Test simplification (not, or, and, with T+Fs)"
  print $ not (and [a_or_na])
  print $ or [and [a_or_na]]
  print $ and [a_or_na, true, true, true, true]
  print $ and [a_or_na, true, true, true, true, false]
  print $ or [and [a_or_na, true, true, true, true], false, false, false]
  -- putStrLn ""
  -- print $ sat_expr $ implies a_or_b a
  -- print $ sat_expr $ implies n_a_or_b a
  -- print $ sat_expr $ implies a_or_b na
  -- print $ sat_expr $ implies n_a_or_b na
  -- print $ sat_expr $ implies a_or_a a
 where
   a :: DNF
   a = varS "a"
   b :: DNF
   b = varS "b"
   na :: DNF
   na = not a
   a_or_b :: DNF
   a_or_b = or [a, b]
   n_a_or_b :: DNF
   n_a_or_b = not a_or_b
   a_or_na :: DNF
   a_or_na = or [a, not a]
