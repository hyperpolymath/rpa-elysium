;;; SPDX-License-Identifier: PMPL-1.0-or-later
;;; SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
;;;
;;; guix.scm — GNU Guix package definition for RPA Elysium
;;;
;;; Build with: guix build -f guix.scm
;;; Development shell: guix shell -f guix.scm

(use-modules (guix packages)
             (guix build-system cargo)
             (guix download)
             (guix git-download)
             (guix licenses)
             (gnu packages rust))

(package
  (name "rpa-elysium")
  (version "0.1.0")
  (source
    (local-file "." "rpa-elysium-checkout"
      #:recursive? #t
      #:select? (lambda (file stat)
        ;; Exclude build artefacts and VCS metadata
        (not (or (string-suffix? "/target" file)
                 (string-suffix? "/.git" file)
                 (string-suffix? "/node_modules" file))))))
  (build-system cargo-build-system)
  (arguments
    `(#:cargo-inputs
      ;; Dependencies will be populated once crate packaging is resolved.
      ;; For now this serves as the reproducible build scaffold.
      ()))
  (synopsis "Robotic process automation toolkit")
  (description
    "RPA Elysium is a comprehensive robotic process automation toolkit
built in Rust, featuring a WASM plugin system, filesystem automation,
and multi-language bindings via Idris2 ABI and Zig FFI.")
  (home-page "https://github.com/hyperpolymath/rpa-elysium")
  ;; PMPL-1.0-or-later — using mpl2.0 as closest Guix-known equivalent
  (license mpl2.0))
