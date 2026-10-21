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
{{- $tag := .Values.image.tag | default .Chart.AppVersion -}}
{{- printf "%s:%s" .Values.image.repository $tag -}}
{{- end -}}

{{/*
The DaemonSet writes hostPaths, so the namespace needs the privileged Pod
Security Standard. Always the chart's own namespace.
*/}}
{{- define "ergon-shared-cpus.namespace" -}}
{{- .Values.namespace | default .Release.Namespace -}}
{{- end -}}
