;;; github-personal-stats-test.el --- Tests for the Emacs reporter -*- lexical-binding: t; -*-

;;; Commentary:

;; What these check is mostly the boundary: the shape of a pulse, the shape of a
;; journal line, and what is never in either.  The daemon rejects a batch it
;; cannot read and the collector refuses a journal it cannot parse, so a plugin
;; that gets the shape wrong does not degrade — it stops, and takes the day's
;; other sources with it.
;;
;; Run with:
;;
;;   emacs -Q -batch -L . -l test/github-personal-stats-test.el \
;;     -f ert-run-tests-batch-and-exit

;;; Code:

(require 'ert)
(require 'json)
(require 'github-personal-stats)

(defmacro github-personal-stats-test--in-state (&rest body)
  "Run BODY with a throwaway state directory and an empty queue."
  (declare (indent 0))
  `(let* ((github-personal-stats-state-directory
           (make-temp-file "github-personal-stats-test" t))
          (github-personal-stats--queue nil)
          (github-personal-stats--token nil))
     (unwind-protect (progn ,@body)
       (delete-directory github-personal-stats-state-directory t))))

(ert-deftest github-personal-stats-test-batch-is-what-the-daemon-reads ()
  "A batch names the editor and carries pulses as an array."
  (let* ((body (github-personal-stats--batch
                (list (list (cons 'at 1756234567)
                            (cons 'day "2026-08-27")
                            (cons 'ext "el")))))
         (parsed (json-parse-string body :object-type 'alist)))
    (should (equal (alist-get 'editor parsed) "emacs"))
    (let ((pulse (aref (alist-get 'pulses parsed) 0)))
      (should (equal (alist-get 'at pulse) 1756234567))
      (should (equal (alist-get 'day pulse) "2026-08-27"))
      (should (equal (alist-get 'ext pulse) "el")))))

(ert-deftest github-personal-stats-test-a-journal-line-has-no-day-in-it ()
  "The collector takes the day from the file name, and the fields it
reads are exactly these three."
  (let* ((line (github-personal-stats--journal-line
                (list (cons 'at 10) (cons 'day "2026-08-27") (cons 'ext "rs"))))
         (parsed (json-parse-string line :object-type 'alist)))
    (should (equal (mapcar #'car parsed) '(at editor ext)))
    (should (equal (alist-get 'editor parsed) "emacs"))
    (should-not (alist-get 'day parsed))))

(ert-deftest github-personal-stats-test-the-journal-is-appended-not-rewritten ()
  "Two writes leave two lines, in the file the day names.

The daemon appends to the same file, so anything that rewrote it
would throw away whatever the daemon had put there."
  (github-personal-stats-test--in-state
    (let ((pulse (list (cons 'at 1) (cons 'day "2026-08-27") (cons 'ext "el"))))
      (github-personal-stats--write-journal (list pulse))
      (github-personal-stats--write-journal
       (list (cons (cons 'at 2) (cdr pulse)))))
    (let ((file (github-personal-stats--journal-file "2026-08-27")))
      (should (file-exists-p file))
      (with-temp-buffer
        (insert-file-contents file)
        (should (= (length (split-string (string-trim (buffer-string)) "\n")) 2))))))

(ert-deftest github-personal-stats-test-each-day-gets-its-own-file ()
  "A flush spanning midnight writes to both days."
  (github-personal-stats-test--in-state
    (github-personal-stats--write-journal
     (list (list (cons 'at 1) (cons 'day "2026-08-26") (cons 'ext ""))
           (list (cons 'at 2) (cons 'day "2026-08-27") (cons 'ext ""))))
    (should (file-exists-p (github-personal-stats--journal-file "2026-08-26")))
    (should (file-exists-p (github-personal-stats--journal-file "2026-08-27")))))

(ert-deftest github-personal-stats-test-an-extension-the-daemon-would-refuse-is-not-sent ()
  "The daemon accepts lowercase letters, digits and dashes, up to
twenty-four of them.  Anything else counts as time under no
language rather than being sent and rejected."
  (let ((named (lambda (name)
                 (with-temp-buffer
                   (setq buffer-file-name name)
                   (github-personal-stats--extension (current-buffer))))))
    (should (equal (funcall named "/tmp/thing.el") "el"))
    (should (equal (funcall named "/tmp/THING.RS") "rs"))
    (should (equal (funcall named "/tmp/Makefile") ""))
    (should (equal (funcall named "/tmp/thing.tar.gz") "gz"))
    (should (equal (funcall named "/tmp/weird.c++") ""))
    (should (equal (funcall named (concat "/tmp/long." (make-string 25 ?a))) ""))
    (should (equal (github-personal-stats--extension (current-buffer)) ""))))

(ert-deftest github-personal-stats-test-a-pulse-carries-nothing-that-locates-you ()
  "Neither transport may hold a path, a project or a buffer name."
  (github-personal-stats-test--in-state
    (with-temp-buffer
      (setq buffer-file-name "/Users/someone/secret-project/src/main.rs")
      (let* ((pulse (list (cons 'at 1)
                          (cons 'day "2026-08-27")
                          (cons 'ext (github-personal-stats--extension
                                      (current-buffer)))))
             (written (concat (github-personal-stats--batch (list pulse))
                             (github-personal-stats--journal-line pulse))))
        (should (string-match-p "\"rs\"" written))
        (dolist (secret '("someone" "secret-project" "src" "main"))
          (should-not (string-match-p secret written)))))))

(ert-deftest github-personal-stats-test-a-long-silence-is-not-work ()
  "Presence is bounded by the idle cutoff, or a window left open
overnight would report the night."
  (cl-letf (((symbol-function 'github-personal-stats--focused-p) (lambda () t))
            ((symbol-function 'github-personal-stats--idle-seconds) (lambda () 60)))
    (let ((github-personal-stats-idle-seconds 600))
      (should (github-personal-stats--present-p)))
    (let ((github-personal-stats-idle-seconds 30))
      (should-not (github-personal-stats--present-p))))
  (cl-letf (((symbol-function 'github-personal-stats--focused-p) (lambda () nil))
            ((symbol-function 'github-personal-stats--idle-seconds) (lambda () 0)))
    (should-not (github-personal-stats--present-p))))

(ert-deftest github-personal-stats-test-beating-queues-only-while-present ()
  "A beat while away leaves the queue alone."
  (github-personal-stats-test--in-state
    (cl-letf (((symbol-function 'github-personal-stats--present-p) (lambda () nil)))
      (github-personal-stats--beat)
      (should (null github-personal-stats--queue)))
    (cl-letf (((symbol-function 'github-personal-stats--present-p) (lambda () t)))
      (github-personal-stats--beat)
      (should (= (length github-personal-stats--queue) 1))
      (should (alist-get 'at (car github-personal-stats--queue))))))

(ert-deftest github-personal-stats-test-the-queue-is-bounded ()
  "A daemon left down cannot grow the queue without limit, and it is
the oldest pulses that go."
  (github-personal-stats-test--in-state
    (let ((github-personal-stats-max-queued 3))
      (setq github-personal-stats--queue
            (mapcar (lambda (at) (list (cons 'at at) (cons 'day "2026-08-27")
                                       (cons 'ext "")))
                    '(1 2 3 4 5)))
      (github-personal-stats--trim)
      (should (equal (mapcar (lambda (pulse) (alist-get 'at pulse))
                             github-personal-stats--queue)
                     '(3 4 5))))))

(ert-deftest github-personal-stats-test-with-no-daemon-the-journal-takes-it ()
  "The default sink loses nothing on a machine where no daemon runs:
there is no token, so there is nothing to send, and the pulses go
to the journal the collector reads."
  (github-personal-stats-test--in-state
    (setq github-personal-stats--queue
          (list (list (cons 'at 1) (cons 'day "2026-08-27") (cons 'ext "el"))))
    (let ((github-personal-stats-sink 'auto)
          (went nil))
      (github-personal-stats--flush (lambda (where) (setq went where)))
      (should (eq went 'journal))
      (should (null github-personal-stats--queue))
      (should (file-exists-p (github-personal-stats--journal-file "2026-08-27"))))))

(ert-deftest github-personal-stats-test-daemon-only-keeps-what-it-could-not-send ()
  "Asked for the daemon and only the daemon, an unreachable daemon
costs nothing: the pulses wait."
  (github-personal-stats-test--in-state
    (let ((pulses (list (list (cons 'at 1) (cons 'day "2026-08-27") (cons 'ext "el"))))
          (github-personal-stats-sink 'daemon)
          (went 'unset))
      (setq github-personal-stats--queue pulses)
      (github-personal-stats--flush (lambda (where) (setq went where)))
      (should (null went))
      (should (equal github-personal-stats--queue pulses))
      (should-not (file-exists-p (github-personal-stats--journal-file "2026-08-27"))))))

(ert-deftest github-personal-stats-test-a-token-is-sixty-four-characters ()
  "A half written token file is not a token, and using one would send
requests that can only be refused."
  (github-personal-stats-test--in-state
    (let ((file (github-personal-stats--token-file)))
      (write-region "not-a-token\n" nil file nil 'no-message)
      (should-not (github-personal-stats--token))
      (setq github-personal-stats--token nil)
      (write-region (concat (make-string 64 ?a) "\n") nil file nil 'no-message)
      (should (equal (github-personal-stats--token) (make-string 64 ?a))))))

(ert-deftest github-personal-stats-test-closing-emacs-keeps-the-last-pulses ()
  "A request cannot be waited for on the way out, so what is left goes
to the journal, which the collector reads just the same."
  (github-personal-stats-test--in-state
    (setq github-personal-stats--queue
          (list (list (cons 'at 1) (cons 'day "2026-08-27") (cons 'ext "el"))))
    (github-personal-stats--keep)
    (should (null github-personal-stats--queue))
    (should (file-exists-p (github-personal-stats--journal-file "2026-08-27")))))

(ert-deftest github-personal-stats-test-turning-the-mode-off-keeps-them-too ()
  "Stopping cancels the timers that would have delivered the queue, so
stopping has to deliver it."
  (github-personal-stats-test--in-state
    (setq github-personal-stats--queue
          (list (list (cons 'at 1) (cons 'day "2026-08-27") (cons 'ext "el"))))
    (github-personal-stats--stop)
    (should (null github-personal-stats--queue))
    (should (file-exists-p (github-personal-stats--journal-file "2026-08-27")))))

(ert-deftest github-personal-stats-test-the-daemon-only-sink-never-writes-a-file ()
  "The setting is honoured on the way out as well: asked for the daemon
and only the daemon, teardown tries the daemon rather than
quietly writing the journal the setting rules out."
  (github-personal-stats-test--in-state
    (let ((github-personal-stats-sink 'daemon)
          (pulses (list (list (cons 'at 1) (cons 'day "2026-08-27") (cons 'ext "el")))))
      (setq github-personal-stats--queue pulses)
      (github-personal-stats--stop)
      (should-not (file-exists-p (github-personal-stats--journal-file "2026-08-27")))
      ;; Nothing could be sent without a token, so they are still here.
      (should (equal github-personal-stats--queue pulses)))))

(provide 'github-personal-stats-test)
;;; github-personal-stats-test.el ends here
