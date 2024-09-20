# Changelog

This changelog track changes to the qollage project starting at version v0.1.0

## 0.3.1

* Use ureq's `native-certs` feature.

## 0.3.0

* Changed the number formatting to not display `0.00` if the number is < 0.01
* Added test for `SqrtPauliY` and `InvSqrtPauliY`.
* Fixed a release bug.

## 0.2.2

* Fixed a bug where a gate that has an optional circuit would sometimes fail to be rendered (e.g. PragmaGetStateVector)
* Improved spacing to avoid text overlap.
* Added support for GateDefinition and CalledDefinedGate

## 0.2.1

* Fixed a build error

## 0.2.0

* Updated to qoqo 1.15 with support for `SqrtPauliY` and `InvSqrtPauliY`.
* Export render_typst_str in roqollage to allow other packages to use the typst compilation.

## 0.1.0

* Initial commit.
