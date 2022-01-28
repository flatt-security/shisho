---
title: 'Getting Started'
metaTitle: 'Getting Started - Shisho Cloud'
metaDescription: 'This page describes details of Shisho Cloud preparation.'
---

## Overview

Shisho Cloud supports you in monitoring your cloud resources to prevent security matters. All you need to do is just three steps:

1. Sign up 
2. Connect with repository managers
3. Link with repositories

### 1. Sign up

Shisho Cloud supports three SSO, [GitHub](https://github.com/), [GitLab](https://about.gitlab.com/) and [BitBucket](https://bitbucket.org/product). Yes, this equals the currently supported repository managers. It might be better to sign up with one of them, which is your preferred repository connection. For instance, if you want to connect with one of GitHub repositories, you should sign up with GitHub SSO. After sign-up, let's start the registration with easy steps!

<img src="/images/sso.png" alt="sso screenshot" width="400"/>

### 2. Select repository manager 

First of all, you need to select one of the repository connections. The currently supported services are:

1. [GitHub](https://github.com/)
2. [GitLab](https://about.gitlab.com/)
3. [BitBucket](https://bitbucket.org/product)

> 📝 Tips: For GitHub users, you need to install the Shisho GitHub App, which is one of the official "GitHub Apps". It supports integrating with Shisho Cloud and managing access permissions. If you have any questions, plase check "Shisho GitHub App" on the page [Frequently asked questions](/shisho-cloud/frequently-asked-questions) for the further details.

### 3. Select repository 

Please select a target repository that Shisho Cloud monitors your Terraform code to maintain your healthy cloud resources. If you do not have Terraform code OR you want to test Shisho Cloud without your own repositories, please folk and select a provided test repository. The Terraform code in the test repository misconfigures AWS resources and policies for dummy resources on purpose. We assume it is enough to demonstrate the performance of Shisho Cloud.

> 📝 Tips: If you have some questions about the test repository, please check the section "[flatt-security/tfgoat-aws](https://github.com/flatt-security/tfgoat-aws)" on the page [Frequently asked questions](/shisho-cloud/frequently-asked-questions)

That's all you have to do. Let's work and develop as usual with Terraform code which is monitored by Shisho Cloud!

## Do you have any questions?

Please check the page [Frequently asked questions](/shisho-cloud/frequently-asked-questions)

