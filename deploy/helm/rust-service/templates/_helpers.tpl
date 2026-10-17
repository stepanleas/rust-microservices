{{- define "rust-service.name" -}}
{{- default .Chart.Name .Values.name | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "rust-service.labels" -}}
app.kubernetes.io/name: {{ include "rust-service.name" . }}
app.kubernetes.io/part-of: rust-microservices
app.kubernetes.io/managed-by: {{ .Release.Service }}
helm.sh/chart: {{ .Chart.Name }}-{{ .Chart.Version }}
{{- end -}}

{{- define "rust-service.selectorLabels" -}}
app.kubernetes.io/name: {{ include "rust-service.name" . }}
app.kubernetes.io/part-of: rust-microservices
{{- end -}}

{{- define "rust-service.image" -}}
{{- $global := (.Values.global | default dict).image | default dict -}}
{{- $repository := .Values.image.repository -}}
{{- if not $repository -}}
{{- $prefix := list ($global.registry | default "") ($global.namespace | default "artellas") | compact | join "/" -}}
{{- $repository = printf "%s/rust-%s" $prefix (include "rust-service.name" .) -}}
{{- end -}}
{{- printf "%s:%s" $repository (.Values.image.tag | default $global.tag | default "latest") -}}
{{- end -}}

{{- /* "true" when a KEDA ScaledObject replaces the HorizontalPodAutoscaler. */ -}}
{{- define "rust-service.keda" -}}
{{- if and .Values.autoscaling.enabled .Values.autoscaling.keda.enabled (.Capabilities.APIVersions.Has "keda.sh/v1alpha1/ScaledObject") -}}
true
{{- end -}}
{{- end -}}

{{- define "rust-service.pullPolicy" -}}
{{- $global := (.Values.global | default dict).image | default dict -}}
{{- .Values.image.pullPolicy | default $global.pullPolicy | default "IfNotPresent" -}}
{{- end -}}
