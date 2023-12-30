package main

import (
	"celestia/common"
	"celestia/graph"
	"context"
	"encoding/hex"
	"errors"
	"flag"
	"fmt"
	"log"
	"net/http"
	"os"

	"github.com/99designs/gqlgen/graphql/handler"
	"github.com/99designs/gqlgen/graphql/playground"
	openrpc "github.com/rollkit/celestia-openrpc"
	"github.com/rollkit/celestia-openrpc/types/share"
)

const defaultPort = "8081"

func NewCelestiaDA(cfg common.DAConfig) (*common.CelestiaDA, error) {
	daClient, err := openrpc.NewClient(context.Background(), cfg.Rpc, cfg.AuthToken)
	if err != nil {
		return nil, err
	}

	if cfg.NamespaceId == "" {
		return nil, errors.New("namespace id cannot be blank")
	}
	nsBytes, err := hex.DecodeString(cfg.NamespaceId)
	if err != nil {
		return nil, err
	}

	namespace, err := share.NewBlobNamespaceV0(nsBytes)
	if err != nil {
		return nil, err
	}

	return &common.CelestiaDA{
		Cfg:       cfg,
		Client:    daClient,
		Namespace: namespace,
	}, nil
}

func main() {
	namespace := flag.String("namespace", "000008e5f679bf7116cb", "target namespace")
	auth := flag.String("auth", "", "auth token (default is $CELESTIA_NODE_AUTH_TOKEN)")

	flag.Parse()

	// Check if datain is provided
	if *auth == "" {
		fmt.Println("Please supply auth token")
		return
	}

	fmt.Println("here")
	// Start Celestia DA
	daConfig := common.DAConfig{
		Rpc:         "http://localhost:26658",
		NamespaceId: *namespace,
		AuthToken:   *auth,
	}

	celestiaDA, err := NewCelestiaDA(daConfig)
	if err != nil {
		fmt.Println(err)
		fmt.Println("Error creating Celestia client")
		return
	}
	port := os.Getenv("PORT")
	if port == "" {
		port = defaultPort
	}

	srv := handler.NewDefaultServer(graph.NewExecutableSchema(
		graph.Config{
			Resolvers: &graph.Resolver{
				CelestiaDA: celestiaDA,
			}}))

	http.Handle("/", playground.Handler("GraphQL playground", "/query"))
	http.Handle("/query", srv)

	log.Printf("connect to http://localhost:%s/ for GraphQL playground", port)
	log.Fatal(http.ListenAndServe(":"+port, nil))
}
