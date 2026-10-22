// SPDX-License-Identifier: Apache-2.0

package config

import (
	"context"
	"crypto/tls"
	"crypto/x509"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net"
	"net/http"
	"net/url"
	"os"
	"time"
)

// InClusterNodeLabels GETs one Node and returns its labels.
// It reads the in-cluster address and the service-account token.
// It does not watch the node and it does not write a label.
func InClusterNodeLabels(ctx context.Context, nodeName string) (map[string]string, error) {
	host := os.Getenv("KUBERNETES_SERVICE_HOST")
	port := os.Getenv("KUBERNETES_SERVICE_PORT")
	if host == "" || port == "" {
		return nil, errors.New("config: node API unavailable: kubernetes service address is empty")
	}
	token, err := os.ReadFile("/var/run/secrets/kubernetes.io/serviceaccount/token")
	if err != nil {
		return nil, fmt.Errorf("config: node API unavailable: %w", err)
	}
	caPEM, err := os.ReadFile("/var/run/secrets/kubernetes.io/serviceaccount/ca.crt")
	if err != nil {
		return nil, fmt.Errorf("config: node API unavailable: %w", err)
	}
	pool := x509.NewCertPool()
	if !pool.AppendCertsFromPEM(caPEM) {
		return nil, errors.New("config: node API unavailable: ca.crt has no certificates")
	}
	client := &http.Client{
		Timeout: 10 * time.Second,
		Transport: &http.Transport{
			TLSClientConfig: &tls.Config{RootCAs: pool, MinVersion: tls.VersionTLS12},
		},
	}
	return fetchNodeLabels(ctx, client, "https://"+net.JoinHostPort(host, port), string(token), nodeName)
}

func fetchNodeLabels(ctx context.Context, client *http.Client, baseURL, token, nodeName string) (map[string]string, error) {
	if nodeName == "" {
		return nil, errors.New("config: NODE_NAME is empty; a profile cannot be selected")
	}
	u, err := url.Parse(baseURL)
	if err != nil || u.Scheme == "" || u.Host == "" {
		return nil, errors.New("config: node API unavailable: address is not a URL")
	}
	u.Path = "/api/v1/nodes/" + url.PathEscape(nodeName)
	u.RawQuery = ""
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, u.String(), nil)
	if err != nil {
		return nil, fmt.Errorf("config: node API unavailable: %w", err)
	}
	if token != "" {
		req.Header.Set("Authorization", "Bearer "+token)
	}
	req.Header.Set("Accept", "application/json")
	resp, err := client.Do(req)
	if err != nil {
		return nil, fmt.Errorf("config: node API unavailable: %w", err)
	}
	defer resp.Body.Close()
	body, err := io.ReadAll(io.LimitReader(resp.Body, 1<<20))
	if err != nil {
		return nil, fmt.Errorf("config: node API unavailable: %w", err)
	}
	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("config: node API unavailable: %s", resp.Status)
	}
	var doc struct {
		Metadata struct {
			Labels map[string]string `json:"labels"`
		} `json:"metadata"`
	}
	if err := json.Unmarshal(body, &doc); err != nil {
		return nil, fmt.Errorf("config: node API unavailable: %w", err)
	}
	if doc.Metadata.Labels == nil {
		return map[string]string{}, nil
	}
	return doc.Metadata.Labels, nil
}
