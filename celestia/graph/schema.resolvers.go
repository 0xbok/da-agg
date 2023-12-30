package graph

// This file will be automatically regenerated based on the schema, any resolver implementations
// will be copied through when generating and any unknown code will be moved to the end.
// Code generated by github.com/99designs/gqlgen version v0.17.42

import (
	"celestia/graph/model"
	"context"
	"encoding/hex"
	"errors"
	"strconv"
)

// Submit is the resolver for the submit field.
func (r *mutationResolver) Submit(ctx context.Context, data string, namespace string, authToken string) (*model.SubmitResponse, error) {
	if data == "" {
		return nil, errors.New("empty data")
	}
	datain := []byte(data)
	commitment, height, err := r.CelestiaDA.Store(context.Background(), datain)
	if err != nil {
		return nil, err
	}

	commitmentstr := hex.EncodeToString(commitment)
	hstring := strconv.FormatUint(height, 10)

	return &model.SubmitResponse{
		Commitment: &commitmentstr,
		Height:     &hstring,
	}, nil
}

// Read is the resolver for the read field.
func (r *queryResolver) Read(ctx context.Context, commitment string, height string) (*model.ReadResponse, error) {
	commitmentBytes, err := hex.DecodeString(commitment)
	if err != nil {
		return nil, err
	}
	h, err := strconv.ParseUint(height, 10, 64)
	if err != nil {
		return nil, err
	}
	data, err := r.CelestiaDA.Read(context.Background(), commitmentBytes, h)
	if err != nil {
		return nil, err
	}

	dataout := string(data)
	return &model.ReadResponse{
		Data: &dataout,
	}, nil
}

// Mutation returns MutationResolver implementation.
func (r *Resolver) Mutation() MutationResolver { return &mutationResolver{r} }

// Query returns QueryResolver implementation.
func (r *Resolver) Query() QueryResolver { return &queryResolver{r} }

type mutationResolver struct{ *Resolver }
type queryResolver struct{ *Resolver }
