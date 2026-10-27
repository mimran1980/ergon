{{/*
SPDX-License-Identifier: Apache-2.0
*/}}

{{- define "ergon-shared-cpus.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "ergon-shared-cpus.fullname" -}}
{{- if .Values.fullnameOverride -}}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- include "ergon-shared-cpus.name" . -}}
{{- end -}}
{{- end -}}

{{- define "ergon-shared-cpus.labels" -}}
app.kubernetes.io/name: {{ include "ergon-shared-cpus.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
helm.sh/chart: {{ printf "%s-%s" .Chart.Name .Chart.Version }}
{{- end -}}

{{- define "ergon-shared-cpus.selectorLabels" -}}
app: {{ include "ergon-shared-cpus.name" . }}
{{- end -}}

{{- define "ergon-shared-cpus.image" -}}
{{- if .Values.image.digest -}}
{{- if not (hasPrefix "sha256:" .Values.image.digest) -}}
{{- fail "image.digest must be a sha256 digest (sha256:...)" -}}
{{- end -}}
{{- printf "%s@%s" .Values.image.repository .Values.image.digest -}}
{{- else -}}
{{- $tag := .Values.image.tag | default .Chart.AppVersion -}}
{{- printf "%s:%s" .Values.image.repository $tag -}}
{{- end -}}
{{- end -}}

{{/*
hostContract mounts are an existing directory or file. FileOrCreate would
invent a kubelet config the plugin would then trust.
*/}}
{{- define "ergon-shared-cpus.hostPathType" -}}
{{- $t := . | default "Directory" -}}
{{- if or (eq $t "FileOrCreate") (eq $t "DirectoryOrCreate") -}}
{{- fail "hostContract hostPath type must be File or Directory; FileOrCreate is refused" -}}
{{- end -}}
{{- if and (ne $t "File") (ne $t "Directory") -}}
{{- fail (printf "hostContract hostPath type %q must be File or Directory" $t) -}}
{{- end -}}
{{ $t }}
{{- end -}}

{{/*
The DaemonSet writes hostPaths, so the namespace needs the privileged Pod
Security Standard. Always the chart's own namespace.
*/}}
{{- define "ergon-shared-cpus.namespace" -}}
{{- .Values.namespace | default .Release.Namespace -}}
{{- end -}}

{{/*
Operator fields of one configuration. Used for the flat file and for each
named profile. It does not emit cgroupRoot, profiles, or nodeProfileLabel.
*/}}
{{- define "ergon-shared-cpus.configBody" -}}
sharedCpus: {{ .sharedCpus | quote }}
oversubscription: {{ .oversubscription | default 1 }}
namespaces: [{{ range $i, $n := .namespaces }}{{ if $i }}, {{ end }}{{ $n | quote }}{{ end }}]
defaultAffinity: {{ .defaultAffinity | default "shared" | quote }}
hookPath: {{ .hookPath | default "/opt/ergon/bin/ergon-cpus" | quote }}
annotationOptIn: {{ .annotationOptIn | default false }}
metricsAddress: {{ .metricsAddress | default ":9466" | quote }}
devRoot: {{ .devRoot | default "/host/dev" | quote }}
{{- $io := "off" }}
{{- if .seccomp }}
{{- $io = .seccomp.ioUring | default "off" }}
{{- end }}
seccomp:
  ioUring: {{ $io | quote }}
seccompProfilePath: {{ .seccompProfilePath | default "/etc/ergon-shared-cpus/ergon-io-uring.json" | quote }}
schedFifo: {{ .schedFifo | default "off" | quote }}
ptp: {{ .ptp | default "off" | quote }}
hugepagesAeronDir: {{ .hugepagesAeronDir | default "" | quote }}
{{- with .rlimits }}
{{- if or .memlock .nofile .rtPrio .nice }}
rlimits:
{{- if .memlock }}
  memlock: {{ .memlock | quote }}
{{- end }}
{{- if .nofile }}
  nofile: {{ .nofile | quote }}
{{- end }}
{{- if .rtPrio }}
  rtPrio: {{ .rtPrio | quote }}
{{- end }}
{{- if .nice }}
  nice: {{ .nice | quote }}
{{- end }}
{{- end }}
{{- end }}
{{- end -}}
