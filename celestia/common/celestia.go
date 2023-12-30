package common

import (
	"context"
	"errors"

	openrpc "github.com/rollkit/celestia-openrpc"
	"github.com/rollkit/celestia-openrpc/types/blob"
	"github.com/rollkit/celestia-openrpc/types/share"
)

type DAConfig struct {
	Rpc         string `koanf:"rpc"`
	NamespaceId string `koanf:"namespace-id"`
	AuthToken   string `koanf:"auth-token"`
}

type CelestiaDA struct {
	Cfg       DAConfig
	Client    *openrpc.Client
	Namespace share.Namespace
}

func (c *CelestiaDA) Store(ctx context.Context, message []byte) ([]byte, uint64, error) {
	dataBlob, err := blob.NewBlobV0(c.Namespace, message)
	if err != nil {
		return nil, 0, err
	}
	commitment, err := blob.CreateCommitment(dataBlob)
	if err != nil {
		return nil, 0, err
	}
	height, err := c.Client.Blob.Submit(ctx, []*blob.Blob{dataBlob}, openrpc.DefaultSubmitOptions())
	if err != nil {
		return nil, 0, err
	}
	if height == 0 {
		return nil, 0, errors.New("unexpected response code")
	}

	return commitment, height, nil
}

func (c *CelestiaDA) Read(ctx context.Context, commitment []byte, height uint64) ([]byte, error) {

	blob, err := c.Client.Blob.Get(ctx, height, c.Namespace, commitment)
	if err != nil {
		return nil, err
	}

	return blob.Data, nil
}
