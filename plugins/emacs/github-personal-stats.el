;;; github-personal-stats.el --- Report coding activity to your own record -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Liu Chong

;; Author: Liu Chong <mail@liuchong.dev>
;; Version: 1.5.0
;; Package-Requires: ((emacs "27.1"))
;; Keywords: convenience, tools
;; URL: https://github.com/liuchong/github-personal-stats

;;; Commentary:

;; Reports time at the editor to the same local record the other plugins write
;; to, so that hours spent in Emacs are counted beside hours spent anywhere else.
;;
;; What leaves this process is a timestamp, a local date and a file extension.
;; No path, no project name, no buffer contents, no repository.  This file is the
;; only part of the pipeline that ever sees a path, and it keeps it.
;;
;; What it reports is presence rather than typing.  A day spent directing an
;; agent produces almost no keystrokes in a buffer, and a plugin measuring
;; keystrokes reports such a day as empty; asking instead whether you were at the
;; editor is true of every way of working.  Presence is bounded by an idle
;; timeout, because Emacs is habitually left in front of you: see
;; `github-personal-stats-idle-seconds'.
;;
;; It does not compute hours, decide what a language is, or publish anything.
;; Pulses are moments; turning moments into sessions, and sessions into a
;; published record, is the collector's job and is done once for every source
;; rather than once per editor.  Two transports carry them:
;;
;;   daemon   POST to the local daemon, which journals them and can show them
;;            in its panel straight away.
;;   journal  Append them to the daemon's own journal on disk, which needs no
;;            daemon, no port and no token: whenever the collector next runs on
;;            this machine it reads them like any others.
;;
;; The default is to prefer the daemon and fall back to the journal, so a machine
;; with no daemon still records everything.
;;
;; Usage:
;;
;;   (require 'github-personal-stats)
;;   (github-personal-stats-mode 1)

;;; Code:

(require 'json)
(require 'subr-x)
(require 'url)

(defconst github-personal-stats-version "1.5.0"
  "Version reported to the daemon.

Deliberately the project's version rather than the plugin's own: a
plugin whose only job is to speak one protocol has nothing to say
that the release it shipped in does not.")

(defgroup github-personal-stats nil
  "Report time at the editor to your own activity record."
  :group 'tools
  :prefix "github-personal-stats-")

(defcustom github-personal-stats-sink 'auto
  "Where pulses go.

`auto' prefers the daemon and writes to the journal when it cannot
be reached, which loses nothing on a machine where no daemon runs.
`daemon' and `journal' pick one and stay with it."
  :type '(choice (const :tag "Daemon, falling back to the journal" auto)
                 (const :tag "Daemon only" daemon)
                 (const :tag "Journal file only" journal))
  :group 'github-personal-stats)

(defcustom github-personal-stats-daemon-url "http://127.0.0.1:7391"
  "Where the local daemon listens."
  :type 'string
  :group 'github-personal-stats)

(defcustom github-personal-stats-state-directory nil
  "Directory holding the token and the pulse journal.

Nil means the same place every other part of the project looks:
`XDG_STATE_HOME'/github-personal-stats, or
~/.local/state/github-personal-stats when that is unset."
  :type '(choice (const :tag "The usual place" nil) directory)
  :group 'github-personal-stats)

(defcustom github-personal-stats-pulse-seconds 30
  "How often presence becomes a pulse.

Must stay well under the collector's idle timeout, or the gap
between two pulses stops counting as time at the editor."
  :type 'integer
  :group 'github-personal-stats)

(defcustom github-personal-stats-flush-seconds 60
  "How often queued pulses are sent or written.

Batched so that a morning's work is a handful of requests rather
than one per pulse."
  :type 'integer
  :group 'github-personal-stats)

(defcustom github-personal-stats-idle-seconds 600
  "How long without input still counts as being at the editor.

The plugin cannot see you, only whether Emacs has been given
anything to do.  A window left in front of you all night would
otherwise report the night as work; this bounds that error to ten
minutes by default.  Reading and directing an agent both produce
input, so the cutoff does not cost a session spent doing either."
  :type 'integer
  :group 'github-personal-stats)

(defcustom github-personal-stats-max-queued 2000
  "How many unsent pulses to keep.

Bounded so that a daemon left down cannot grow the queue without
limit.  The oldest go first: recent work matters more."
  :type 'integer
  :group 'github-personal-stats)

(defconst github-personal-stats--editor "emacs"
  "How this plugin names itself to the daemon.")

(defconst github-personal-stats--safe-extension "\\`[a-z0-9-]\\{1,24\\}\\'"
  "What the daemon accepts as a file extension.
Anything else is reported as no extension at all, which still
counts as time under an unknown kind of file.")

(defvar github-personal-stats--queue nil
  "Pulses waiting to be sent, oldest first.")

(defvar github-personal-stats--token nil
  "The daemon's token, once read.  Forgotten when it is refused.")

(defvar github-personal-stats--pulse-timer nil)
(defvar github-personal-stats--flush-timer nil)
(defvar github-personal-stats--wrote-to-journal nil
  "Whether the last flush fell back to the journal, for the lighter.")

;;;; Where things live

(defun github-personal-stats-state-directory ()
  "The directory holding the token and the journal."
  (or github-personal-stats-state-directory
      (expand-file-name "github-personal-stats"
                        (or (getenv "XDG_STATE_HOME")
                            (expand-file-name "~/.local/state")))))

(defun github-personal-stats--token-file ()
  (expand-file-name "token" (github-personal-stats-state-directory)))

(defun github-personal-stats--journal-file (day)
  (expand-file-name (format "pulses/%s.jsonl" day)
                    (github-personal-stats-state-directory)))

(defun github-personal-stats--token ()
  "The daemon's token, read from disk once and remembered.

A token is sixty-four hex characters; anything else is a partly
written file or a leftover, and treating it as a token would send
requests that can only be refused."
  (or github-personal-stats--token
      (setq github-personal-stats--token
            (let ((file (github-personal-stats--token-file)))
              (when (file-readable-p file)
                (let ((found (with-temp-buffer
                               (insert-file-contents file)
                               (string-trim (buffer-string)))))
                  (when (= (length found) 64) found)))))))

;;;; What a pulse says

(defun github-personal-stats--today ()
  "The local date, as this machine sees it.

Reported rather than derived later, so that a journal read under a
changed timezone still lands in the day the work happened."
  (format-time-string "%Y-%m-%d"))

(defun github-personal-stats--extension (buffer)
  "The kind of file BUFFER holds, if it holds a file at all.

A shell, a magit status or a help buffer is not a file and reports
no extension, which counts as time under an unknown kind rather
than being dropped: you were at the editor either way."
  (let* ((name (buffer-file-name buffer))
         (found (and name (downcase (or (file-name-extension name) "")))))
    (if (and found (string-match-p github-personal-stats--safe-extension found))
        found
      "")))

(defun github-personal-stats--focused-p ()
  "Whether any frame has the window system's focus.

A terminal frame cannot answer this, and neither can a frame of a
daemon nobody has connected to; both report `unknown', which is
taken as focused and left to the idle cutoff to bound."
  (or (not (fboundp 'frame-focus-state))
      (let ((focused nil))
        (dolist (frame (frame-list))
          (when (frame-focus-state frame)
            (setq focused t)))
        focused)))

(defun github-personal-stats--idle-seconds ()
  "How long Emacs has been without input."
  (let ((idle (current-idle-time)))
    (if idle (float-time idle) 0)))

(defun github-personal-stats--present-p ()
  "Whether this counts as being at the editor."
  (and (github-personal-stats--focused-p)
       (< (github-personal-stats--idle-seconds)
          github-personal-stats-idle-seconds)))

(defun github-personal-stats--beat ()
  "Record one instant of being at the editor."
  (when (github-personal-stats--present-p)
    (setq github-personal-stats--queue
          (append github-personal-stats--queue
                  (list (list (cons 'at (floor (float-time)))
                              (cons 'day (github-personal-stats--today))
                              (cons 'ext (github-personal-stats--extension
                                          (window-buffer (selected-window))))))))
    (github-personal-stats--trim)))

(defun github-personal-stats--trim ()
  "Drop the oldest pulses once the queue is full."
  (let ((excess (- (length github-personal-stats--queue)
                   github-personal-stats-max-queued)))
    (when (> excess 0)
      (setq github-personal-stats--queue
            (nthcdr excess github-personal-stats--queue)))))

;;;; Sending

(defun github-personal-stats--batch (pulses)
  "PULSES as the body the daemon reads."
  (let ((json-encoding-pretty-print nil))
    (json-encode `((editor . ,github-personal-stats--editor)
                   (pulses . ,(vconcat pulses))))))

(defun github-personal-stats--journal-line (pulse)
  "PULSE as one line of the daemon's own journal.

The day is the journal's file name rather than a field, which is
how the collector reads it back."
  (let ((json-encoding-pretty-print nil))
    (json-encode `((at . ,(alist-get 'at pulse))
                   (editor . ,github-personal-stats--editor)
                   (ext . ,(or (alist-get 'ext pulse) ""))))))

(defun github-personal-stats--write-journal (pulses)
  "Append PULSES to the journal, one file per day.

Appending rather than rewriting is what makes this safe to do
while the daemon is doing the same thing to the same file: the
journal is the record of what was observed, and nothing rewrites
it."
  (dolist (day (delete-dups (mapcar (lambda (pulse) (alist-get 'day pulse)) pulses)))
    (let ((file (github-personal-stats--journal-file day))
          (lines ""))
      (dolist (pulse pulses)
        (when (equal (alist-get 'day pulse) day)
          (setq lines (concat lines (github-personal-stats--journal-line pulse) "\n"))))
      (make-directory (file-name-directory file) t)
      (let ((coding-system-for-write 'utf-8-unix))
        (write-region lines nil file t 'no-message))))
  t)

(defun github-personal-stats--post (path body handler)
  "POST BODY to PATH on the daemon and call HANDLER with the status.

HANDLER receives nil when the daemon could not be reached at all,
which is a different answer from a daemon that refused: one is
worth trying again and the other is final."
  (let* ((token (github-personal-stats--token))
         (url-request-method "POST")
         (url-request-extra-headers
          `(("Content-Type" . "application/json")
            ("Authorization" . ,(concat "Bearer " (or token "")))))
         (url-request-data (encode-coding-string body 'utf-8)))
    (if (not token)
        (funcall handler nil)
      (condition-case nil
          (url-retrieve
           (concat (string-trim-right github-personal-stats-daemon-url "/") path)
           (lambda (_status)
             ;; Unbound or nil when nothing answered, which is the difference
             ;; between a daemon that is down and a daemon that said no.
             (let ((code (and (boundp 'url-http-response-status)
                              url-http-response-status)))
               (when (buffer-live-p (current-buffer)) (kill-buffer))
               (funcall handler code)))
           nil t t)
        (error (funcall handler nil))))))

(defun github-personal-stats--flush (&optional then)
  "Send or write the queued pulses, then call THEN with what happened.

THEN receives `daemon', `journal' or nil.  A refusal from the
daemon is final and the pulses are dropped, except for a rejected
token, which is worth reading again and retrying: the token is
replaced whenever the daemon's state is rebuilt."
  (let ((pulses github-personal-stats--queue))
    (cond
     ((null pulses) (when then (funcall then nil)))
     ((eq github-personal-stats-sink 'journal)
      (setq github-personal-stats--queue nil)
      (github-personal-stats--write-journal pulses)
      (setq github-personal-stats--wrote-to-journal t)
      (when then (funcall then 'journal)))
     (t
      (setq github-personal-stats--queue nil)
      (github-personal-stats--post
       "/v1/pulses" (github-personal-stats--batch pulses)
       (lambda (code)
         (cond
          ((and code (<= 200 code) (< code 300))
           (setq github-personal-stats--wrote-to-journal nil)
           (when then (funcall then 'daemon)))
          ((eq code 401)
           (setq github-personal-stats--token nil)
           (github-personal-stats--requeue pulses)
           (when then (funcall then nil)))
          ((and code (<= 400 code) (< code 500))
           (message "github-personal-stats: the daemon refused the pulses (%s)" code)
           (when then (funcall then nil)))
          ((eq github-personal-stats-sink 'auto)
           (github-personal-stats--write-journal pulses)
           (setq github-personal-stats--wrote-to-journal t)
           (when then (funcall then 'journal)))
          (t
           (github-personal-stats--requeue pulses)
           (when then (funcall then nil))))))))))

(defun github-personal-stats--requeue (pulses)
  "Put PULSES back at the front of the queue."
  (setq github-personal-stats--queue (append pulses github-personal-stats--queue))
  (github-personal-stats--trim))

(defun github-personal-stats--announce ()
  "Tell the daemon this plugin is loaded.

A plugin loaded into a window nobody is looking at produces no
pulses, which is indistinguishable from a plugin that never
loaded, and the difference is the first thing anyone wants to
know."
  (when (memq github-personal-stats-sink '(auto daemon))
    (github-personal-stats--post
     "/v1/hello"
     (let ((json-encoding-pretty-print nil))
       (json-encode `((editor . ,github-personal-stats--editor)
                      (version . ,github-personal-stats-version))))
     #'ignore)))

;;;; Commands

;;;###autoload
(defun github-personal-stats-send-now ()
  "Send whatever is queued, and say where it went."
  (interactive)
  (setq github-personal-stats--token nil)
  (let ((waiting (length github-personal-stats--queue)))
    (github-personal-stats--flush
     (lambda (where)
       (message "github-personal-stats: %s"
                (pcase where
                  ('daemon (format "%d pulses sent to %s" waiting
                                   github-personal-stats-daemon-url))
                  ('journal (format "%d pulses written to %s" waiting
                                    (github-personal-stats--journal-file
                                     (github-personal-stats--today))))
                  (_ (if (github-personal-stats--token)
                         (format "%d pulses still waiting" waiting)
                       (format "no token at %s; is the daemon running?"
                               (github-personal-stats--token-file))))))))))

;;;###autoload
(defun github-personal-stats-status ()
  "Describe what is being reported and where."
  (interactive)
  (message
   (string-join
    (list (format "sink       %s" github-personal-stats-sink)
          (format "daemon     %s" github-personal-stats-daemon-url)
          (format "token      %s" (if (github-personal-stats--token)
                                      (github-personal-stats--token-file)
                                    (format "missing at %s"
                                            (github-personal-stats--token-file))))
          (format "journal    %s" (github-personal-stats--journal-file
                                   (github-personal-stats--today)))
          (format "queued     %d pulses" (length github-personal-stats--queue))
          (format "present    %s" (if (github-personal-stats--present-p)
                                      "yes"
                                    (format "no, idle %.0fs"
                                            (github-personal-stats--idle-seconds)))))
    "\n")))

;;;; The mode

(defun github-personal-stats--lighter ()
  "What the mode line says: enough to tell working from stuck."
  (cond ((github-personal-stats--queue-stuck-p) " stats!")
        (github-personal-stats--wrote-to-journal " stats~")
        (t " stats")))

(defun github-personal-stats--queue-stuck-p ()
  "Whether pulses are piling up rather than going anywhere."
  (> (length github-personal-stats--queue)
     (max 4 (/ (* 2 github-personal-stats-flush-seconds)
               (max 1 github-personal-stats-pulse-seconds)))))

;;;###autoload
(define-minor-mode github-personal-stats-mode
  "Report time at this editor to your own activity record."
  :global t
  :lighter (:eval (github-personal-stats--lighter))
  :group 'github-personal-stats
  (if github-personal-stats-mode
      (github-personal-stats--start)
    (github-personal-stats--stop)))

(defun github-personal-stats--start ()
  (github-personal-stats--stop)
  (setq github-personal-stats--pulse-timer
        (run-at-time t (max 5 github-personal-stats-pulse-seconds)
                     #'github-personal-stats--beat))
  (setq github-personal-stats--flush-timer
        (run-at-time t (max 5 github-personal-stats-flush-seconds)
                     (lambda () (github-personal-stats--flush))))
  (github-personal-stats--announce)
  (add-hook 'kill-emacs-hook #'github-personal-stats--keep))

(defun github-personal-stats--stop ()
  (when github-personal-stats--pulse-timer
    (cancel-timer github-personal-stats--pulse-timer)
    (setq github-personal-stats--pulse-timer nil))
  (when github-personal-stats--flush-timer
    (cancel-timer github-personal-stats--flush-timer)
    (setq github-personal-stats--flush-timer nil))
  (remove-hook 'kill-emacs-hook #'github-personal-stats--keep)
  ;; The timers that would have delivered what is queued have just been
  ;; cancelled, so turning the mode off has to deal with the queue itself or the
  ;; last minute of work goes nowhere.
  (github-personal-stats--keep))

(defun github-personal-stats--keep ()
  "Save what has not been sent, by whatever means the sink allows.

Closing Emacs and turning the mode off have the same problem: a
request cannot be waited for, and nothing will run afterwards to
retry it.  Writing to the journal is synchronous and is read by
the collector just the same, so that is the answer wherever the
sink permits it.

Asked for the daemon and only the daemon, the journal is not
written and one last attempt is made instead: an unreachable
daemon then costs these pulses, which is the choice that setting
makes."
  (when github-personal-stats--queue
    (if (eq github-personal-stats-sink 'daemon)
        (github-personal-stats--flush)
      (ignore-errors
        (github-personal-stats--write-journal github-personal-stats--queue)
        (setq github-personal-stats--queue nil)))))

(provide 'github-personal-stats)
;;; github-personal-stats.el ends here
