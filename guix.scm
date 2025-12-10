;; rpa-elysium - Guix Package Definition
;; Run: guix shell -D -f guix.scm

(use-modules (guix packages)
             (guix gexp)
             (guix git-download)
             (guix build-system gnu)
             ((guix licenses) #:prefix license:)
             (gnu packages base))

(define-public rpa_elysium
  (package
    (name "rpa-elysium")
    (version "0.1.0")
    (source (local-file "." "rpa-elysium-checkout"
                        #:recursive? #t
                        #:select? (git-predicate ".")))
    (build-system gnu-build-system)
    (synopsis "Guix channel/infrastructure")
    (description "Guix channel/infrastructure - part of the RSR ecosystem.")
    (home-page "https://github.com/hyperpolymath/rpa-elysium")
    (license license:agpl3+)))

;; Return package for guix shell
rpa_elysium
